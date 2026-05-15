#!/bin/bash
set -e

echo "========================================================"
echo "    Tesseract OS: Custom Kernel & Initramfs Synthesis   "
echo "========================================================"

# 0. Check host dependencies
for cmd in git make gcc flex bison bc cpio xz docker; do
    if ! command -v $cmd &> /dev/null; then
        echo "Error: Required command '$cmd' is not installed."
        echo "Please install it (e.g., sudo apt install git build-essential flex bison bc cpio xz-utils docker.io)"
        exit 1
    fi
done

# We will store intermediate massive files in a .cache subfolder
WORKDIR="$(pwd)/.cache/custom_kernel"
mkdir -p "$WORKDIR"

# 1. Fetch Linux Kernel
echo ""
echo "[1/4] Fetching Stable Linux Kernel (v7.0.7) ..."
cd "$WORKDIR"
KERNEL_TAG="v7.0.7"
KERNEL_REPO="https://git.kernel.org/pub/scm/linux/kernel/git/stable/linux.git"

if ! git ls-remote --exit-code --tags "$KERNEL_REPO" "refs/tags/${KERNEL_TAG}" >/dev/null; then
    echo "ERROR: Kernel tag ${KERNEL_TAG} not found in ${KERNEL_REPO}" >&2
    exit 1
fi

if [ ! -d "linux" ]; then
    git clone --depth=1 --branch "$KERNEL_TAG" "$KERNEL_REPO" linux
else
    echo "linux/ already exists; verifying it matches ${KERNEL_TAG} before reusing it."
fi

# 2. Configure and Build Kernel
echo ""
echo "[2/4] Compiling Custom Linux Kernel (bzImage) ..."
cd linux

git describe --tags --exact-match || {
    echo "ERROR: checked-out kernel is not exactly ${KERNEL_TAG}" >&2
    exit 1
}
# Generate default config
make defconfig
# Enable essential drivers for Tesseract OS (DRM, Vulkan AMD/NVIDIA, Sound)
scripts/config -e DRM -e DRM_AMDGPU -e DRM_NOUVEAU -e SOUND -e SND_HDA_INTEL -e BPF_SYSCALL
make olddefconfig

# Compile the kernel (this will take time!)
make -j$(nproc) bzImage
KERNEL_IMG="$(pwd)/arch/x86/boot/bzImage"

if [ ! -f "$KERNEL_IMG" ]; then
    echo "Error: bzImage compilation failed."
    exit 1
fi

# 3. Build Minimal Initramfs using Docker
echo ""
echo "[3/4] Synthesizing Tesseract OS Initramfs ..."
cd ../../.. # Back to project root

# Build the docker container with the minimal environment
docker build -t tesseract-initramfs -f Dockerfile.initramfs .

echo "Exporting filesystem from container..."
CONTAINER_ID=$(docker create tesseract-initramfs)
docker export $CONTAINER_ID > "$WORKDIR/rootfs.tar"
docker rm $CONTAINER_ID

echo "Converting root filesystem to cpio.gz (RAM disk format) ..."
mkdir -p "$WORKDIR/rootfs"
cd "$WORKDIR/rootfs"
# Clear previous if it exists
rm -rf *
tar -xf ../rootfs.tar

# Repackage the flat filesystem into the cpio format the Linux kernel expects
find . | cpio -o -H newc | gzip -9 > ../initramfs.cpio.gz
INITRAMFS_IMG="$WORKDIR/initramfs.cpio.gz"

# 4. Final Output
echo ""
echo "[4/4] Synthesis Complete! Copying final artifacts..."
cd ../../..
cp "$KERNEL_IMG" tesseract-kernel-bzImage
cp "$INITRAMFS_IMG" tesseract-initramfs.cpio.gz

echo "========================================================"
echo "SUCCESS: Tesseract OS is now merged with a pure Linux Kernel."
echo "Artifacts generated in project root:"
echo " - tesseract-kernel-bzImage"
echo " - tesseract-initramfs.cpio.gz"
echo ""
echo "To test the zero-overhead boot sequence using QEMU, run:"
echo "qemu-system-x86_64 -kernel tesseract-kernel-bzImage -initrd tesseract-initramfs.cpio.gz -m 4G -enable-kvm"
echo "========================================================"
