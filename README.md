# fuwasdr

wideband receiver from scratch

## memory mapping

- 0x10000000 (2M): FLASH
  - 0x0..0x1cc000: program text
  - 0x1cc000..0x1ce000 (8k): large font
  - 0x1ce000..0x200000 (200k): misaki font
