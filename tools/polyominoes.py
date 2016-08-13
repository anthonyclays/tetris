#!/usr/bin/env python3
# -*- coding: utf-8 -*-

from itertools import chain

def all_polyominoes(n):
    if n <= 1:
        return [((0, 0),)]
    polys = set()
    for poly in all_polyominoes(n - 1):
        all_blocks = set(chain.from_iterable(neighbours(block) for block in poly)) - set(poly)
        for block in all_blocks:
            polys.add(canonical((*poly, block)))
    return sorted(polys)

def neighbours(block):
    (x, y) = block
    return (x+1, y), (x, y+1), (x-1, y), (x, y-1)

def normalize(poly):
    minx = min(x for (x, _) in poly)
    miny = min(y for (_, y) in poly)
    return tuple(sorted((x - minx, y - miny) for (x, y) in poly))

def rotate(poly):
    return tuple((-y, x) for (x, y) in poly)

def versions(poly):
    yield normalize(poly)
    for _ in range(3):
        poly = normalize(rotate(poly))
        yield poly

def canonical(poly):
    return min(versions(poly))

def print_poly(poly):
    print('Poly', poly)
    maxx = max(x for (x, _) in poly)
    maxy = max(y for (_, y) in poly)
    for y in range(maxy+1):
        for x in range(maxx+1):
            print('#' if (x, y) in poly else ' ', end='')
        print()

def generate_code(N, name):
    polys = all_polyominoes(N)
    print('    #[cfg(feature="{name}")]'.format(name=name))
    print('    pub use self::{name}::*;'.format(name=name))
    print('    #[cfg(feature="{name}")]'.format(name=name))
    print('    mod {name} {{'.format(name=name))
    print('        pub const POLYOMINO_FORCE: f32 = TODO;')
    print('        pub const POLYOMINO_ANG_FORCE: f32 = TODO;')
    print('        pub const POLYOMINOS: [[[usize; 2]; {}]; {}] = ['.format(N, len(polys)))
    for poly in polys:
        print('            [{}],'.format(', '.join('[{}, {}]'.format(*block) for block in poly)))
    print('        ];')
    print('    }')
    print()

if __name__ == "__main__":
    import sys
    try:
        [_, N, name] = sys.argv
        N = int(N)
        generate_code(N, name)
    except:
        for (N, name) in zip(range(3, 11), ['triominos', 'tetrominos', 'pentominos', 'hexominos', 'heptominos', 'octominos', 'nonominos', 'decominos']):
            generate_code(N, name)
