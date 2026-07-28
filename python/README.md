# Python implementations for generated MAY components

The files under `output/python` are generated abstract component classes. This
directory contains concrete implementations and executable examples.

From the repository root:

```sh
python3 python/run_examples.py
python3 -m unittest discover -s python -p 'test_*.py' -v
```

If generated sources are missing or stale, regenerate them first:

```sh
cd may_rust
cargo run -- -i ../examples/all/ex1 -o ../output/python --keep-intermediate
cd ..
```

`python/ex1/Start.py` is the Python-native service contract used by the
implementations. It is intentionally separate from `examples/all/ex1/Start.java`;
no Java-to-Python transpilation is performed.
