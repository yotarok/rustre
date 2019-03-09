Rust regular expression
=======================

The goal of this project is to implement thrax-like [^1] rewriter in rust,
where both EDSL and engine are implemented in rust language.

Currently, this reporsitory has the follwing features:

1. Regular expression compiler
2. Some basic FST operations
   (concat/union/rm-epsilon/closure/shortest-distance/arcsort/determinize)
3. grep-like utility tool for demonstrating regexp features

and future enhancement will be done for:

4. More FST operations (minimize, pushing, etc...)
5. Implement rewriting tool like sed
6. Rust-based EDSL for defining rewriter

[^1] Thrax: http://www.openfst.org/twiki/bin/view/GRM/Thrax

Benchmark on GREP
=================

The throughputs are evaluated by searching `(([02468][13579]){5})+` from the test
input (~1 GB). The test input is generated by repeating first 50 million digits
from pi 20 times.

The throughputs include computation time for regular expression compilation and
optimization.

| grep (macOS) | Regengrep [^1] | ripgrep [^2] | rustre-grep |
| ------------ | -------------- | ------------ | ----------- |
| 15.2 MB/s    | 232.1 MB/s     | 231.3 MB/s   | 254.4 MB/s  |

For mitigating the overhead for output, outputs of those commands are piped to
`wc` command.
For Regen, computation time for preoptimization of regular expression was
dominant so the result here was computed with lower optimization level (-O0)
which led to the best result. Parallelization is disabled for focusing on the
single-thread performance.

Command lines used for evaluation is as follows:

```sh
# For grep (macOS)
$ time grep '\([02468][13579][02468][13579][02468][13579][02468][13579][02468][13579]\)+' ./pi50Mx20.txt | wc

# For regengrep
$ time DYLD_LIBRARY_PATH=$HOME/src/Regen/src/bin ~/src/Regen/src/bin/regengrep -O0 '(([0-4][5-9]){5})+' ~/proj/rustre/pi50Mx20.txt | wc

# For ripgrep
$ time rg '(([02468][13579]){5})+' ./pi50Mx20.txt | wc

# For rustre-grep
$ time ./target/release/rustre -e '(([02468][13579]){5})+' -J ./pi50Mx20.txt | wc
```

[^1] Regen: https://github.com/sinya8282/Regen
[^2] ripgrep: https://github.com/BurntSushi/ripgrep