#!/usr/bin/env python3
"""Exact independent explicit-queue / total-created-hands split oracle.

Uses the frozen module's independent rational *test* oracle for ordinary play;
no production C++ or compact F(k,s) table is imported. Never writes bytecode in
the frozen dependency directory.
"""
import csv
from fractions import Fraction as F
from functools import lru_cache
import importlib.util
from pathlib import Path
import subprocess
import sys

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("ordinary_reference", ROOT / "tests/ev_exact_oracle.py")
ordinary = importlib.util.module_from_spec(spec)
spec.loader.exec_module(ordinary)
RANKS = (1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 10, 10, 10)

@lru_cache(None)
def play(pair, second, up, h17, das):
    total, usable = ordinary.hand(pair, second)
    # Split aces are forced stand, including A+A and A+ten. No natural bonus.
    if pair == 1:
        return ordinary.stand(total, up, h17)
    value = ordinary.best(total, usable, up, h17)
    return max(value, ordinary.double(total, usable, up, h17)) if das else value

@lru_cache(None)
def queue_value(queue, created, up, h17, das, maximum=4, forced=False):
    assert len(queue) <= created <= maximum
    if not queue:
        return F(0)
    pair, rest = queue[0], queue[1:]
    result = F(0)
    for second in RANKS:
        value = play(pair, second, up, h17, das) + queue_value(rest, created, up, h17, das, maximum, forced)
        if second == pair and pair != 1 and created < maximum:
            alternative = queue_value((pair, pair) + rest, created + 1, up, h17, das, maximum, forced)
            value = alternative if forced else max(value, alternative)
        result += value / 13
    return result

def matching_policy(k, s, pair, up, h17, das):
    if pair == 1:
        return 0  # Not applicable: no ace resplit decisions exist.
    if not s:
        return 1  # Cap reached: ordinary play only.
    created = 4 - s
    ordinary_branch = play(pair, pair, up, h17, das) + queue_value((pair,) * (k - 1), created, up, h17, das)
    resplit_branch = queue_value((pair,) * (k + 1), created + 1, up, h17, das)
    if abs(ordinary_branch - resplit_branch) <= F(2, 10**8):
        return 3
    return 2 if resplit_branch > ordinary_branch else 1

def main():
    if len(sys.argv) != 2:
        raise SystemExit("usage: exact_queue_oracle.py ORACLE_DUMP_EXECUTABLE")
    output = subprocess.check_output([str(Path(sys.argv[1]).resolve())], text=True)
    cases = policies = 0
    maximum_error = F(0)
    for row in csv.reader(output.splitlines()):
        h17, das, pair, up = map(int, row[:4])
        reference = queue_value((pair, pair), 2, up, h17, das)
        actual = F.from_float(float(row[4]))
        error = abs(reference - actual)
        assert error <= F(1, 10**8), (row[:5], reference, float(error))
        maximum_error = max(maximum_error, error)
        no_resplit = queue_value((pair, pair), 2, up, h17, das, maximum=2)
        three_hands = queue_value((pair, pair), 2, up, h17, das, maximum=3)
        forced = queue_value((pair, pair), 2, up, h17, das, forced=True)
        assert reference >= three_hands >= no_resplit
        assert reference >= forced
        assert F(-8) <= reference <= F(8)
        expected_policies = [matching_policy(k, s, pair, up, h17, das)
                             for s in range(3) for k in range(1, 5 - s)]
        assert list(map(int, row[5:])) == expected_policies, (row, expected_policies)
        policies += len(expected_policies)
        cases += 1
    assert cases == 400
    # Published mismatching cells: a *separate* forced-resplit policy reproduces
    # their rounded values. This is evidence for a policy distinction, not proof
    # of the publisher's full undocumented internal strategy.
    published_forced_controls = ((5, 10, -.729969), (10, 6, .468117),
                                 (10, 10, -.371278), (9, 7, .364837))
    for pair, up, printed in published_forced_controls:
        forced = queue_value((pair, pair), 2, up, False, True, forced=True)
        optimal = queue_value((pair, pair), 2, up, False, True)
        assert abs(forced - F.from_float(printed)) <= F(500001, 10**12)
        assert optimal > forced
        print(f"Published-policy control {pair},{pair} vs {up}: "
              f"forced={float(forced):.17g}; optional optimum={float(optimal):.17g}; printed={printed:.6f}")
    print(f"PASS exact queue oracle: {cases} initial pair/upcard/H17/S17/DAS cells; "
          f"{policies} matching-draw policy checks; cap2<=cap3<=cap4 and optimum>=forced; "
          f"max absolute binary64 error={float(maximum_error):.17g} (bound=1e-8)")

if __name__ == "__main__":
    main()
