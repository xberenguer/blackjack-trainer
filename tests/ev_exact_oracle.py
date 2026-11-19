#!/usr/bin/env python3
"""Independent rational oracle: usable-ace transitions and recursive Fraction EV.

Unlike the production implementation this has no minimum-sum DAG or binary arithmetic.
It shares no production code. Every compared reference is an exact rational.
"""
from fractions import Fraction as F
from functools import lru_cache
from pathlib import Path
import csv
import subprocess
import sys

def draw(total, usable, rank):
    if rank == 1 and total + 11 <= 21:
        return total + 11, True
    total += rank
    if total > 21 and usable:
        return total - 10, False
    return total, usable

def hand(a, b):
    total, usable = draw(0, False, a)
    return draw(total, usable, b)

@lru_cache(None)
def finish(total, usable, h17):
    if total > 21:
        return (F(1),) + (F(0),) * 5
    if total >= 17 and not (h17 and total == 17 and usable):
        return tuple(F(i == total - 16) for i in range(6))
    values = [F(0)] * 6
    # Enumerate 13 equiprobable ranks; the ten-valued branch appears four times.
    for rank in (1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 10, 10, 10):
        child = finish(*draw(total, usable, rank), h17)
        for i in range(6):
            values[i] += child[i] / 13
    return tuple(values)

@lru_cache(None)
def dealer(up, h17, cleared):
    outcomes = [F(0)] * 7
    allowed = []
    for hole in (1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 10, 10, 10):
        blackjack = (up == 1 and hole == 10) or (up == 10 and hole == 1)
        if cleared and blackjack:
            continue
        allowed.append((hole, blackjack))
    for hole, blackjack in allowed:
        if blackjack:
            outcomes[6] += F(1, len(allowed))
        else:
            child = finish(*hand(up, hole), h17)
            for i in range(6):
                outcomes[i] += child[i] / len(allowed)
    assert sum(outcomes) == 1
    return tuple(outcomes)

@lru_cache(None)
def stand(total, up, h17):
    if total > 21:
        return F(-1)
    p = dealer(up, h17, True)
    return p[0] + sum(p[t - 16] * ((total > t) - (total < t)) for t in range(17, 22))

@lru_cache(None)
def hit(total, usable, up, h17):
    return sum((best(*draw(total, usable, rank), up, h17)
                for rank in (1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 10, 10, 10)), F(0)) / 13

@lru_cache(None)
def best(total, usable, up, h17):
    if total > 21:
        return F(-1)
    if total == 21:
        return stand(total, up, h17)
    return max(stand(total, up, h17), hit(total, usable, up, h17))

def double(total, usable, up, h17):
    return sum((2 * stand(draw(total, usable, rank)[0], up, h17)
                for rank in (1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 10, 10, 10)), F(0)) / 13

def main():
    if len(sys.argv) != 2:
        raise SystemExit("usage: exact_oracle.py PATH_TO_ORACLE_DUMP")
    output = subprocess.check_output([str(Path(sys.argv[1]).resolve())], text=True)
    comparisons = actions = distributions = 0
    maximum = F(0)
    for row in csv.reader(output.splitlines()):
        kind, h17, up = row[0], bool(int(row[1])), int(row[2])
        if kind == "D":
            refs = dealer(up, h17, bool(int(row[3])))
            actual = row[4:]
            distributions += 1
        else:
            a, b = int(row[3]), int(row[4])
            total, usable = hand(a, b)
            refs = (stand(total, up, h17), hit(total, usable, up, h17),
                    double(total, usable, up, h17), F(-1, 2))
            actual = row[5:]
            actions += 1
        assert len(actual) == len(refs)
        for value, reference in zip(actual, refs):
            error = abs(F.from_float(float(value)) - reference)
            assert error <= F(1, 10**10), (row, reference, float(error))
            maximum = max(maximum, error)
            comparisons += 1
    assert actions == 520 and distributions == 40
    print(f"PASS exact rational oracle: {actions} complete hard/soft/upcard/H17/S17 action cells; "
          f"{distributions} pre/postpeek distributions; {comparisons} exact comparisons; "
          f"max absolute binary64 error={float(maximum):.17g} (bound=1e-10)")

if __name__ == "__main__":
    main()
