#!/bin/bash

# Make sure PWD is set to script's directory
cd "${0%/*}"

# Prepare environment
if uname -a | grep -q "Linux"; then
    os="linux"
else
    echo "UNKNOWN OS"
    exit 1
fi
version="$(grep "^version"  ../common/Cargo.toml | grep -o "[0-9.]*")"
build_dir="$PWD/../target/release"
dst_dir="$PWD/release"
package_name="check_mate_""$os""_$version"

# Print environment
echo "os: $os"
echo "version: $version"
echo "build_dir: $build_dir"
echo "dst_dir: $dst_dir"
echo "package_name: $package_name"


printf "\n---------------------- Compiling\n"
cargo build --release || exit 1

printf "\n---------------------- Zipping\n"
mkdir -p "$dst_dir/$package_name" || exit 1
cd "$dst_dir" || exit 1
cp "$build_dir/check_mate_client" "$package_name/" || exit 1
cp "$build_dir/check_mate_server" "$package_name/" || exit 1
zip "$package_name.zip" $package_name/* || exit 1
rm -rf "$package_name" || exit 1

printf "\n----------------------\n"
echo "Zip archive created: $(realpath "$package_name.zip")"
zip -sf "$package_name.zip"
checksum="$(sha256sum $package_name.zip | cut -d' ' -f1)"
echo "SHA256: $checksum"

# Update checksum in PKGBUILD (for Arch Linux)
sed -i "s/sha256sums=('.*')/sha256sums=('$checksum')/g" ../PKGBUILD
