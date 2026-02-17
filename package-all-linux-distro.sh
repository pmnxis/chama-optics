#!/bin/bash
# SPDX-FileCopyrightText: © 2025 Jinwoo Park (pmnxis@gmail.com)
# SPDX-License-Identifier: CC0-1.0
#
# package-all-linux-distro.sh — Multi-distro Linux build & package via Proxmox LXC containers
#
# Builds chama-optics across ALL supported Linux distributions simultaneously
# using Proxmox LXC containers (VMIDs 400–406). This script is intended to
# be run from a local machine (e.g. Mac) and connects to Proxmox via SSH.
#
# For building on your current Linux environment, use: bash build-linux.sh
#
# Usage: bash package-all-linux-distro.sh [git-ref]
#
# Prerequisites:
#   - SSH config: "ssh proxmox" connects to Proxmox host
#   - LXC containers created and set up (create-builders.sh, setup-linux.sh)
#   - SSH keys distributed to each LXC

set -e
cd "$(dirname "$0")/.."

GIT_REF="${1:-main}"
VERSION=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/' | sed 's/-rc//')
REPO="https://github.com/pmnxis/chama-optics.git"
DIST_DIR="$(pwd)/dist"
mkdir -p "$DIST_DIR"

# ============================================================================
# Per-distro build configuration
# ============================================================================
#
# Each distro has been tested and verified with specific conditions.
# The build script will display this information and ask for confirmation
# before proceeding.
#
# VMID | Distro        | Tested     | Cargo Features                       | Notes
# -----|---------------|------------|--------------------------------------|---------------------------
# 400  | Debian 12     | 2026-02-18 | desktop,libheif,embedded_libheif     | Standard build
# 401  | Debian 13     | 2026-02-18 | desktop,libheif,embedded_libheif     | Standard build
# 402  | Ubuntu 22.04  | 2026-02-18 | desktop,libheif,embedded_libheif     | libdav1d-dev MUST be removed
# 403  | Ubuntu 24.04  | 2026-02-18 | desktop,libheif,embedded_libheif     | Standard build
# 404  | Fedora 41     | 2026-02-18 | desktop,libheif,embedded_libheif     | Standard build
# 405  | Rocky 9       | 2026-02-18 | desktop,libheif,embedded_libheif     | freetype built from source
# 406  | Arch Linux    | 2026-02-18 | desktop,libheif                      | System libheif (no embedded)
# ============================================================================

# Builder definitions: VMID:hostname:type:features:notes
BUILDERS=(
    "400:debian12:deb:desktop,libheif,embedded_libheif:Standard build"
    "401:debian13:deb:desktop,libheif,embedded_libheif:Standard build"
    "402:ubuntu2204:deb:desktop,libheif,embedded_libheif:libdav1d-dev must be removed (API incompatibility)"
    "403:ubuntu2404:deb:desktop,libheif,embedded_libheif:Standard build"
    "404:fedora41:rpm:desktop,libheif,embedded_libheif:Standard build"
    "405:rocky9:rpm:desktop,libheif,embedded_libheif:freetype 2.13.3 built from source (system version too old)"
    "406:arch:tar:desktop,libheif:System libheif (embedded_libheif disabled due to x264 link issue)"
)

# Tested dates per distro (last verified build date)
declare -A TESTED_DATES=(
    [debian12]="2026-02-18"
    [debian13]="2026-02-18"
    [ubuntu2204]="2026-02-18"
    [ubuntu2404]="2026-02-18"
    [fedora41]="2026-02-18"
    [rocky9]="2026-02-18"
    [arch]="2026-02-18"
)

# Tested OS versions (exact versions at test time)
declare -A TESTED_VERSIONS=(
    [debian12]="Debian 12 (Bookworm)"
    [debian13]="Debian 13 (Trixie)"
    [ubuntu2204]="Ubuntu 22.04 LTS (Jammy Jellyfish)"
    [ubuntu2404]="Ubuntu 24.04 LTS (Noble Numbat)"
    [fedora41]="Fedora 41"
    [rocky9]="Rocky Linux 9"
    [arch]="Arch Linux (rolling)"
)

# ============================================================================
# Display build plan and get confirmation
# ============================================================================

echo "==========================================================================="
echo "  chama-optics v${VERSION} Multi-Distro Build Plan (ref: ${GIT_REF})"
echo "==========================================================================="
echo ""
echo "The following builds will be executed:"
echo ""

for builder in "${BUILDERS[@]}"; do
    IFS=':' read -r vmid name pkg_type features notes <<< "$builder"
    tested="${TESTED_DATES[$name]}"
    version="${TESTED_VERSIONS[$name]}"
    echo "  [VMID $vmid] $version"
    echo "           Tested: $tested"
    echo "           Features: $features"
    echo "           Package: .$pkg_type"
    echo "           Notes: $notes"
    echo ""
done

echo "Output directory: $DIST_DIR"
echo "==========================================================================="
echo ""

# ============================================================================
# Ubuntu 22.04 special warning: libdav1d-dev removal
# ============================================================================

echo "==========================================================================="
echo "  !! WARNING: Ubuntu 22.04 (VMID 402) requires special handling !!"
echo "==========================================================================="
echo ""
echo "  Ubuntu 22.04's system libdav1d-dev provides an OLD API (dav1d 0.9.x)"
echo "  that is INCOMPATIBLE with embedded libheif's expected dav1d API."
echo ""
echo "  Specifically, the struct field 'Dav1dSettings.n_threads' does not exist"
echo "  in Ubuntu 22.04's dav1d headers (it uses 'n_tile_threads' instead)."
echo ""
echo "  To resolve this, 'libdav1d-dev' WILL BE REMOVED from the Ubuntu 22.04"
echo "  container before building. This means the resulting binary will NOT"
echo "  have dav1d (AV1) decode support via libheif on Ubuntu 22.04."
echo ""
echo "  This script will run:"
echo "    apt-get remove -y libdav1d-dev"
echo "  on VMID 402 (ubuntu2204) before starting the build."
echo ""
echo "==========================================================================="
echo ""

read -rp "Do you accept the Ubuntu 22.04 libdav1d-dev removal and wish to proceed with ALL builds? [y/N] " confirm
if [[ ! "$confirm" =~ ^[Yy]$ ]]; then
    echo "Aborted by user."
    exit 1
fi

echo ""
echo "Starting builds..."
echo ""

# ============================================================================
# Build functions
# ============================================================================

get_lxc_ip() {
    local vmid="$1"
    ssh proxmox "pct exec $vmid -- hostname -I 2>/dev/null" | awk '{print $1}'
}

pre_build_hook() {
    local vmid="$1"
    local name="$2"

    case "$name" in
        ubuntu2204)
            echo "  [$name] Removing libdav1d-dev (API incompatibility with embedded libheif)..."
            ssh proxmox "pct exec $vmid -- bash -c '
                apt-get remove -y libdav1d-dev 2>/dev/null || true
            '"
            echo "  [$name] libdav1d-dev removed."
            ;;
        ubuntu2204|rocky9)
            # freetype source build is handled by setup-linux.sh
            # Just ensure PKG_CONFIG_PATH is set
            echo "  [$name] Ensuring freetype PKG_CONFIG_PATH is set..."
            ssh proxmox "pct exec $vmid -- bash -c '
                if [ -f /etc/profile.d/freetype-local.sh ]; then
                    source /etc/profile.d/freetype-local.sh
                fi
            '"
            ;;
    esac
}

build_on_lxc() {
    local vmid="$1"
    local name="$2"
    local pkg_type="$3"
    local features="$4"
    local notes="$5"

    echo "=== [$name] Starting build (VMID $vmid) ==="
    echo "    Features: $features"
    echo "    Notes: $notes"

    # Ensure LXC is running
    ssh proxmox "pct status $vmid | grep -q running || pct start $vmid"
    sleep 2

    # Run pre-build hooks (distro-specific fixups)
    pre_build_hook "$vmid" "$name"

    # Clone/update source
    ssh proxmox "pct exec $vmid -- bash -c '
        source /root/.cargo/env 2>/dev/null
        if [ -d /root/chama-optics ]; then
            cd /root/chama-optics
            git fetch origin
            git checkout ${GIT_REF}
            git pull origin ${GIT_REF} 2>/dev/null || true
        else
            git clone --branch ${GIT_REF} --depth 1 ${REPO} /root/chama-optics
        fi
    '"

    # Build with per-distro features
    ssh proxmox "pct exec $vmid -- bash -c '
        source /root/.cargo/env
        if [ -f /etc/profile.d/freetype-local.sh ]; then
            source /etc/profile.d/freetype-local.sh
        fi
        cd /root/chama-optics
        cargo build --release --features \"${features}\"
    '"

    # Package
    case "$pkg_type" in
        deb)
            ssh proxmox "pct exec $vmid -- bash /root/chama-optics/infra/package-deb.sh ${VERSION}"
            # Copy .deb out
            local deb_path
            deb_path=$(ssh proxmox "pct exec $vmid -- find /tmp -name 'chama-optics_*.deb' -type f | head -1")
            ssh proxmox "pct pull $vmid $deb_path /tmp/chama-optics-${name}.deb"
            scp proxmox:/tmp/chama-optics-${name}.deb "$DIST_DIR/"
            ;;
        rpm)
            ssh proxmox "pct exec $vmid -- bash /root/chama-optics/infra/package-rpm.sh ${VERSION}"
            local rpm_path
            rpm_path=$(ssh proxmox "pct exec $vmid -- find /root/rpmbuild/RPMS -name 'chama-optics*.rpm' -type f | head -1")
            ssh proxmox "pct pull $vmid $rpm_path /tmp/chama-optics-${name}.rpm"
            scp proxmox:/tmp/chama-optics-${name}.rpm "$DIST_DIR/"
            ;;
        tar)
            # Arch: tar the binary (uses system libheif, no embedded)
            ssh proxmox "pct exec $vmid -- bash -c '
                mkdir -p /tmp/chama-optics-pkg/usr/bin
                cp /root/chama-optics/target/release/chama-optics /tmp/chama-optics-pkg/usr/bin/
                cd /tmp && tar czf chama-optics-${VERSION}-arch-x86_64.tar.gz -C chama-optics-pkg .
            '"
            ssh proxmox "pct pull $vmid /tmp/chama-optics-${VERSION}-arch-x86_64.tar.gz /tmp/"
            scp proxmox:/tmp/chama-optics-${VERSION}-arch-x86_64.tar.gz "$DIST_DIR/"
            ;;
    esac

    echo "=== [$name] Done ==="
}

# ============================================================================
# Run all builds
# ============================================================================

# Run builds (sequential to avoid overloading, or parallel if enough RAM)
for builder in "${BUILDERS[@]}"; do
    IFS=':' read -r vmid name pkg_type features notes <<< "$builder"
    build_on_lxc "$vmid" "$name" "$pkg_type" "$features" "$notes" &
done

wait

echo ""
echo "==========================================================================="
echo "  All builds complete!"
echo "==========================================================================="
echo ""
ls -lh "$DIST_DIR/"
