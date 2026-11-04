#!/bin/sh
set -eu

cd "$(dirname "$0")/.."

version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
dist=target/dist
app="$dist/derust.app"

rustup target add aarch64-apple-darwin x86_64-apple-darwin >/dev/null
cargo build --release --target aarch64-apple-darwin
cargo build --release --target x86_64-apple-darwin

rm -rf "$dist"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"

lipo -create \
    target/aarch64-apple-darwin/release/derust \
    target/x86_64-apple-darwin/release/derust \
    -output "$app/Contents/MacOS/derust"

sed "s/VERSION/$version/g" assets/Info.plist > "$app/Contents/Info.plist"

iconset="$dist/derust.iconset"
mkdir -p "$iconset"
for size in 16 32 128 256 512; do
    sips -z $size $size assets/icon.png --out "$iconset/icon_${size}x${size}.png" >/dev/null
    double=$((size * 2))
    sips -z $double $double assets/icon.png --out "$iconset/icon_${size}x${size}@2x.png" >/dev/null
done
iconutil -c icns "$iconset" -o "$app/Contents/Resources/derust.icns"
rm -rf "$iconset"

codesign --force --deep --sign - "$app"

ditto -c -k --keepParent "$app" "$dist/derust-$version-macos.zip"

staging="$dist/dmg"
mkdir -p "$staging"
cp -R "$app" "$staging/"
ln -s /Applications "$staging/Applications"
hdiutil create -volname derust -srcfolder "$staging" -ov -format UDZO "$dist/derust-$version-macos.dmg" >/dev/null
rm -rf "$staging"

echo "$dist/derust-$version-macos.dmg"
echo "$dist/derust-$version-macos.zip"
