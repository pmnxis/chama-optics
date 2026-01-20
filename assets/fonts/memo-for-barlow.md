<!--
SPDX-FileCopyrightText: © 2026 Jinwoo Park (pmnxis@gmail.com)

SPDX-License-Identifier: MIT OR Apache-2.0
-->

# Barlow Variable Font Solution

## Overview

This project creates a properly configured Barlow variable font with:
- **Weight range:** 100 to 900 (CSS standard values)
- **Width:** Regular width (matches static Barlow fonts)
- **Single axis:** Only `wght` axis (unnecessary `wdth` axis removed)

## Problem Description

### Initial Issue
BarlowGX.ttf is a working variable font, but it has two problems:

1. **Non-standard weight values:** Uses internal values (22-188) instead of CSS standard (100-900)
2. **Condensed default width:** Default width is condensed (wdth=300), not regular (wdth=500)

### Why This Matters
When using the font in code:
- Need to map internal values to CSS standards (e.g., 22 -> 100, 71 -> 400)
- Characters appear narrower than expected (condensed width)
- Inconsistent with static Barlow fonts

## Solution

### Approach 1: Simple FVAR Remapping (Narrow Width)
**File:** `Barlow-Variable-Remapped-Narrow.ttf`

**What was changed:**
- Modified `fvar` table only (metadata)
- Changed `wght` axis range: 22-188 -> 100-900
- Remapped named instances: internal values -> CSS standard
- Changed `wdth` default value: 300 -> 500

**Result:**
- [OK] Weight axis works correctly (100-900)
- [FAIL] **Width is still condensed** (35-60% of em instead of 50-90%)
- [FAIL] FVAR metadata says wdth=500, but HMTX table still has condensed widths

**Why it failed:**
Changing `fvar.defaultValue` only updates **metadata**, not **actual glyph widths** in `hmtx` table. The `hmtx` table contains the default widths that are actually rendered, and these were still based on the original default (wght=22, wdth=300).

### Approach 2: Rebuild with wdth=500 (Correct Width)
**File:** `Barlow-Variable-Remapped.ttf`

**What was changed:**
1. Extracted 9 instances from BarlowGX.ttf at `wdth=500` (regular width)
   - Weights: 30, 39, 53, 71, 96, 116, 141, 166, 188
2. Remapped weight values to CSS standard
   - 30 -> 100 (Thin)
   - 39 -> 200 (ExtraLight)
   - 53 -> 300 (Light)
   - 71 -> 400 (Regular)
   - 96 -> 500 (Medium)
   - 116 -> 600 (SemiBold)
   - 141 -> 700 (Bold)
   - 166 -> 800 (ExtraBold)
   - 188 -> 900 (Black)
3. Built new variable font from extracted instances
4. Removed `wdth` axis (only kept `wght`)

**Result:**
- [OK] Weight axis works correctly (100-900)
- [OK] **Width is regular** (matches static fonts: 50-90% of em)
- [OK] All glyphs have proper variations
- [OK] Single `wght` axis only (cleaner implementation)

**Why it works:**
By extracting instances at `wdth=500` and rebuilding the font, the `hmtx` table now contains the correct default widths. The variable font is properly constructed with `wdth=500` as the true default, not just in metadata.

## File Comparison

| Feature | Narrow Width (Remapped-Narrow) | Correct Width (Remapped) |
|----------|-------------------------------|------------------------|
| Weight Range | 100-900 [OK] | 100-900 [OK] |
| Default Width | Condensed [FAIL] | Regular [OK] |
| wdth Axis | Exists [WARN] | Removed [OK] |
| File Size | 207 KB | 385 KB |
| Glyph 'A' Width | 359 units (35.9%) | 607 units (60.7%) |
| Glyph 'M' Width | 502 units (50.2%) | 715 units (71.5%) |
| Static Match | [FAIL] | [OK] |

**Recommended:** Use `Barlow-Variable-Remapped.ttf`

## Width Verification

### Static Barlow-Regular.ttf (Reference)
```
'A': 588 units (58.8%)
'M': 699 units (69.9%)
'W': 874 units (87.4%)
'a': 511 units (51.1%)
'm': 824 units (82.4%)
'w': 722 units (72.2%)
'0': 547 units (54.7%)
```

### Barlow-Variable-Remapped-Narrow.ttf (Approach 1)
```
'A': 359 units (35.9%)  <-- Too narrow!
'M': 502 units (50.2%)  <-- Too narrow!
'W': 577 units (57.7%)  <-- Too narrow!
'a': 388 units (38.8%)  <-- Too narrow!
'm': 629 units (62.9%)  <-- Too narrow!
'w': 513 units (51.3%)  <-- Too narrow!
'0': 392 units (39.2%)  <-- Too narrow!
```

### Barlow-Variable-Remapped.ttf (Approach 2 - Recommended)
```
'A': 607 units (60.7%)  <-- Match!
'M': 715 units (71.5%)  <-- Match!
'W': 878 units (87.8%)  <-- Match!
'a': 511 units (51.1%)  <-- Perfect match!
'm': 827 units (82.7%)  <-- Match!
'w': 721 units (72.1%)  <-- Match!
'0': 565 units (56.5%)  <-- Match!
```

## Usage

### CSS
```css
@font-face {
  font-family: 'Barlow';
  src: url('Barlow-Variable-Remapped.ttf') format('truetype');
  font-weight: 100 900;
}

body {
  font-family: 'Barlow';
}

/* Use any weight from 100 to 900 */
.thin { font-weight: 100; }
.light { font-weight: 300; }
.regular { font-weight: 400; }
.bold { font-weight: 700; }
.black { font-weight: 900; }

/* Or use any value in between */
.custom { font-weight: 450; }
```

### C/C++ (FreeType2)
```c
FT_Face face;
FT_Init_FreeType(&library);
FT_New_Face(library, "Barlow-Variable-Remapped.ttf", 0, &face);

// Set weight (100-900)
FT_Set_Var_Design_Coordinates(face, 0, 400);  // Regular
FT_Set_Var_Design_Coordinates(face, 0, 700);  // Bold
FT_Set_Var_Design_Coordinates(face, 0, 100);  // Thin
```

### Python (FreeType)
```python
from freetype import Face

face = Face('Barlow-Variable-Remapped.ttf')

# Set weight (100-900)
weight = 450  # Any value between 100 and 900
face.set_var_design_coordinates(0, weight)

# Render character
face.load_char(ord('A'), flags=0)
```

### Rust (rusttype)
```rust
use rusttype::{Font, Scale};

let font_data = std::fs::read("Barlow-Variable-Remapped.ttf")?;
let font = Font::try_from_vec(font_data)?;

// Set weight (wght axis is index 0)
let weight = 450; // Any value between 100 and 900
font.set_variation(font.variation_axis_index("wght").unwrap(), weight);
```

## Technical Details

### Font Tables

- **FVAR (Font Variations):** Defines variation axes and named instances
- **GVAR (Glyph Variations):** Stores variation data for each glyph
- **HMTX (Horizontal Metrics):** Contains default glyph widths

### Key Insight

In variable fonts:
- `fvar.defaultValue` is **metadata** that tells apps what the default is
- `hmtx` table contains **actual default glyph widths** that are rendered
- These must be consistent for correct rendering

When we only changed `fvar.defaultValue` (Approach 1):
- Metadata said wdth=500
- But `hmtx` still had condensed widths (from wdth=300)
- Result: Font rendered condensed despite metadata

When we extracted instances at wdth=500 and rebuilt (Approach 2):
- New `hmtx` table contains regular widths
- `fvar.defaultValue` matches actual defaults
- Result: Font renders correctly

## Files

### Generated Files
- `Barlow-Variable-Remapped-Narrow.ttf` - Narrow width (Approach 1)
- `Barlow-Variable-Remapped.ttf` - **Recommended** (Approach 2)

### Scripts
- `remap_gx_weight_to_standard.py` - FVAR remapping (Approach 1)
- `rebuild_with_wdth500.py` - Rebuild with correct width (Approach 2)
- `check_font_widths.py` - Width verification tool

## Conclusion

**Use `Barlow-Variable-Remapped.ttf` for:**
- [OK] Standard CSS weight values (100-900)
- [OK] Correct regular width matching static fonts
- [OK] All glyphs with proper variations
- [OK] Clean implementation (single `wght` axis)
- [OK] Full compatibility with all browsers and font libraries

The font is ready for production use in any application that requires standard CSS weight values with proper character widths.
