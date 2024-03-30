from PIL import Image
from pathlib import Path

def main(infile: Path, outfile: Path):
    img = Image.open(infile)

    bw = img.convert('1', dither=Image.NONE)

    width, height = bw.size
    w = width // 16
    h = height // 6

    data = b''
    for i in range(6):
        for j in range(16):
            for y in range(h):
                row = 0
                for x in range(w):
                    if bw.getpixel((j*w+x, i*h+y)):
                        row |= 1 << x
                # data.append(row.to_bytes(1, 'little'))
                data += row.to_bytes(2, 'little')

    with open(outfile, 'wb') as f:
        f.write(data)

import sys
if __name__ == '__main__':
    if len(sys.argv) != 3:
        print(f'Usage: {sys.argv[0]} infile outfile')
        sys.exit(1)
    main(Path(sys.argv[1]), Path(sys.argv[2]))
