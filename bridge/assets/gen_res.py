#!/usr/bin/env python3
"""Generate icon.o — COFF object file with a .rsrc section embedding icon.ico.
Requires no external tools; pure Python stdlib. Produces the same output as
  windres icon.rc -O coff -o icon.o
Run from bridge/assets/ or any directory; icon.ico and icon.o sit next to this file."""

import struct, os, sys

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
ICO_PATH   = os.path.join(SCRIPT_DIR, "icon.ico")
OUT_PATH   = os.path.join(SCRIPT_DIR, "icon.o")

# ── Read .ico ─────────────────────────────────────────────────────────────────

with open(ICO_PATH, 'rb') as f:
    ico = f.read()

_res, _type, count = struct.unpack_from('<HHH', ico, 0)
assert _type == 1, "Not an icon file"

entries = []
for i in range(count):
    off = 6 + i * 16
    w, h, cc, _r, planes, bpp, size, img_off = struct.unpack_from('<BBBBHHII', ico, off)
    entries.append({
        'width':  256 if w == 0 else w,
        'height': 256 if h == 0 else h,
        'color_count': cc, 'planes': planes, 'bit_count': bpp,
        'bytes_in_res': size, 'data': ico[img_off: img_off + size],
        'id': i + 1,
    })
N = len(entries)

# ── Build .rsrc section ───────────────────────────────────────────────────────
#
# Directory tree (all offsets are section-relative):
#
#  off 0      : Root DIR  (16 B header + 2×8 B entries) = 32 B
#  off 32     : RT_ICON DIR  (16 B + N×8 B)
#  off 48+8N  : RT_ICON lang dirs  (N × 24 B each)
#  off 48+32N : RT_GROUP_ICON DIR  (24 B)
#  off 72+32N : RT_GROUP_ICON lang dir  (24 B)
#  off 96+32N : Data entries  ((N+1) × 16 B)
#  off 112+48N: Raw image data + GRPICONDIR

def align4(x): return (x + 3) & ~3

dir_rt_icon_off       = 32
dir_rt_icon_lang_base = 32 + 16 + 8 * N     # 48+8N
dir_rt_grp_off        = 48 + 8 * N + 24 * N # 48+32N
dir_rt_grp_lang_off   = dir_rt_grp_off + 24  # 72+32N
total_dirs            = dir_rt_grp_lang_off + 24  # 96+32N

data_entry_base    = total_dirs                   # first RT_ICON data entry
data_entry_grp_off = data_entry_base + N * 16     # RT_GROUP_ICON data entry
raw_base           = data_entry_grp_off + 16       # start of raw image bytes

# Raw-data offsets for each icon image
img_offs = []
cur = raw_base
for e in entries:
    img_offs.append(cur)
    cur = align4(cur + len(e['data']))

grp_off  = cur
grp_size = 6 + N * 14   # GRPICONDIR + N × GRPICONDIRENTRY
total_sz = align4(grp_off + grp_size)

section = bytearray(total_sz)

def dir_hdr(off, n_id):
    struct.pack_into('<IIHHHH', section, off, 0, 0, 0, 0, 0, n_id)

def dir_entry(off, id_, target, is_dir):
    struct.pack_into('<II', section, off,
        id_ & 0x7FFFFFFF,
        (0x80000000 if is_dir else 0) | (target & 0x7FFFFFFF))

def data_entry(off, rva, size):
    struct.pack_into('<IIII', section, off, rva, size, 0, 0)

# Root: 2 entries (RT_ICON=3, RT_GROUP_ICON=14)
dir_hdr(0, 2)
dir_entry(16, 3,  dir_rt_icon_off, True)
dir_entry(24, 14, dir_rt_grp_off,  True)

# RT_ICON dir: N entries (IDs 1..N)
dir_hdr(dir_rt_icon_off, N)
for i, e in enumerate(entries):
    lang_off = dir_rt_icon_lang_base + i * 24
    dir_entry(dir_rt_icon_off + 16 + i * 8, e['id'], lang_off, True)

# RT_ICON lang dirs
for i, e in enumerate(entries):
    lang_off = dir_rt_icon_lang_base + i * 24
    de_off   = data_entry_base + i * 16
    dir_hdr(lang_off, 1)
    dir_entry(lang_off + 16, 1033, de_off, False)

# RT_GROUP_ICON dir: 1 entry (ID 1)
dir_hdr(dir_rt_grp_off, 1)
dir_entry(dir_rt_grp_off + 16, 1, dir_rt_grp_lang_off, True)

# RT_GROUP_ICON lang dir
dir_hdr(dir_rt_grp_lang_off, 1)
dir_entry(dir_rt_grp_lang_off + 16, 1033, data_entry_grp_off, False)

# Data entries — OffsetToData is section-relative and will be relocated to RVA
for i, e in enumerate(entries):
    data_entry(data_entry_base + i * 16, img_offs[i], len(e['data']))
data_entry(data_entry_grp_off, grp_off, grp_size)

# Raw icon images
for i, e in enumerate(entries):
    section[img_offs[i]: img_offs[i] + len(e['data'])] = e['data']

# GRPICONDIR + GRPICONDIRENTRY[]
grp_data = struct.pack('<HHH', 0, 1, N)
for e in entries:
    w = 0 if e['width']  == 256 else e['width']
    h = 0 if e['height'] == 256 else e['height']
    grp_data += struct.pack('<BBBBHHIh',
        w, h, e['color_count'], 0,
        e['planes'], e['bit_count'], e['bytes_in_res'], e['id'])
section[grp_off: grp_off + len(grp_data)] = grp_data

# ── Relocations ───────────────────────────────────────────────────────────────
# IMAGE_REL_AMD64_ADDR32NB (type 3): converts section-relative offset → RVA.
# One relocation per IMAGE_RESOURCE_DATA_ENTRY.OffsetToData field.
reloc_sites = [data_entry_base + i * 16 for i in range(N)]
reloc_sites.append(data_entry_grp_off)

# ── Assemble COFF file ────────────────────────────────────────────────────────
#
# [0..19]  COFF File Header (20 B)
# [20..59] Section Header ".rsrc" (40 B)
# [60..]   Section data
# [60+sz]  Relocations  ((N+1) × 10 B)
# [reloc+] Symbol table (1 × 18 B)
# [sym+]   String table (4 B)

sec_off   = 60
reloc_off = sec_off + total_sz
sym_off   = reloc_off + (N + 1) * 10
str_off   = sym_off + 18

coff = bytearray()

# COFF File Header: Machine, NumSections, Timestamp, PtrSymTable, NumSymbols, OptHdrSz, Chars
coff += struct.pack('<HHIIIHH', 0x8664, 1, 0, sym_off, 1, 0, 0)

# Section Header: Name[8], VirtSz, VirtAddr, RawSz, PtrRaw, PtrReloc, PtrLines, NumRelocs, NumLines, Flags
# Flags: IMAGE_SCN_CNT_INITIALIZED_DATA(0x40) | IMAGE_SCN_MEM_READ(0x40000000)
coff += struct.pack('<8sIIIIIIHHI',
    b'.rsrc\x00\x00\x00', 0, 0, total_sz, sec_off, reloc_off, 0,
    N + 1, 0, 0x40000040)

coff += bytes(section)

# Relocations
for va in reloc_sites:
    coff += struct.pack('<IIH', va, 0, 3)  # sym_idx=0, IMAGE_REL_AMD64_ADDR32NB

# Symbol table (1 entry for .rsrc section)
coff += struct.pack('<8sIhHBB', b'.rsrc\x00\x00\x00', 0, 1, 0, 3, 0)

# String table (empty: just the 4-byte size field)
coff += struct.pack('<I', 4)

with open(OUT_PATH, 'wb') as f:
    f.write(coff)

print(f"Generated {OUT_PATH}  ({len(coff):,} bytes)")
print(f"  .rsrc: {total_sz} bytes, {N} icon images, {N+1} relocations")
