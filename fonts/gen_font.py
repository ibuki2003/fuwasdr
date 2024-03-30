from pathlib import Path
import sys

GLYPH_H_TABLE = [
    (0x00, 0x01),
    (0x03, 0x05),
    (0x20, 0x27),
    (0x30, 0x31),
    (0x32, 0x34),
    (0x4e, 0xa0),
    (0xff, 0x100),
]

def get_idx(c: int):
    cl = c & 0xff
    c >>= 8
    x = 0
    for a, b in GLYPH_H_TABLE:
        if c < a:
            return None
        elif c <= b:
            return x + (c - a) * 256 + cl

        x += (b - a) * 256

def glyph_tbl_size():
    return sum(b - a for a, b in GLYPH_H_TABLE) * 256

def main(infile: Path, outfile: Path | None):
    buf = [0] * 8 * glyph_tbl_size()
    with infile.open("r") as f:
        char = None
        state = 0
        bbx = (0, 0, 0, 0)
        idx = 0

        for line in f:
            line = line.strip()
            if char is None:
                if line.startswith("ENCODING"):
                    char = int(line.split()[1])
                    state = -1
                    bbx = (0, 0, 0, 0)
                    idx = get_idx(char)
                    print(char, idx)
                    if idx is None:
                        print(f"Invalid character: 0x{char:02X}")
                        idx = 0
                continue

            if line.startswith("ENDCHAR"):
                char = None
                continue

            if state < 0:
                if line.startswith("BBX"):
                    bbx = tuple(map(int, line.split()[1:]))
                elif line.startswith("BITMAP"):
                    state = 0
                continue
            else:
                v = int(line, 16)
                for i in range(bbx[2], 8):
                    k = ((v >> i) & 1) << (7 - i + bbx[2])
                    buf[idx * 8 + state + (8 - bbx[1] - bbx[3] - 2)] |= k
                state += 1
                continue


    if outfile:
        with open(outfile, "wb") as f:
            f.write(bytes(buf))

if __name__ == "__main__":
    inf = Path(sys.argv[1])
    ouf = Path(sys.argv[2]) if len(sys.argv) > 2 else None
    main(inf, ouf)
