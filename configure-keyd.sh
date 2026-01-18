#!/bin/bash
# Configure keyd for extest
# Creates common file and extest.conf for easy maintenance

set -e

KEYD_DIR="/etc/keyd"

die() { echo "Error: $1" >&2; exit 1; }

[ "$EUID" -eq 0 ] || die "Please run with sudo"
[ -f "$KEYD_DIR/default.conf" ] || die "No $KEYD_DIR/default.conf found"

# Backup
cp "$KEYD_DIR/default.conf" "$KEYD_DIR/default.conf.bak"
echo "Backed up to $KEYD_DIR/default.conf.bak"

# Extract [main] and other sections (not [ids]) into common
awk '
    /^\[ids\]/ { in_ids=1; next }
    /^\[/ { in_ids=0 }
    !in_ids
' "$KEYD_DIR/default.conf" > "$KEYD_DIR/common"
echo "Created $KEYD_DIR/common"

# Create new default.conf with [ids] preserved + extest excluded
cat > "$KEYD_DIR/default.conf" << 'EOF'
[ids]
*
-1234:5678
-feed:beef

[main]
include common
EOF
echo "Updated $KEYD_DIR/default.conf"

# Create extest.conf
cat > "$KEYD_DIR/extest.conf" << 'EOF'
[ids]
1234:5678

[main]
include common

# Override keys that should behave differently for Deskflow
# Example: if local keyboard remaps escape but remote has real escape key
escape = escape
` = `
EOF
echo "Created $KEYD_DIR/extest.conf"

# Reload
systemctl restart keyd
echo "Reloaded keyd"

echo ""
echo "Done! Edit $KEYD_DIR/common for shared mappings."
echo "Edit $KEYD_DIR/extest.conf for Deskflow-specific overrides."
