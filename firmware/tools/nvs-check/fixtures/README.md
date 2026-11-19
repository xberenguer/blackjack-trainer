# Synthetic migration fixture

`save-v7-synthetic.bin` starts from the existing published V5 test fixture, then uses the independently specified V7 byte layout: three extra seats/player seat2, enabled ramp units1/3/5/9/13, six categories with exact64-bit totals above2^53. It contains no device data or hidden cards. CRC uses Python zlib. Newer firmware must migrate every field exactly.
