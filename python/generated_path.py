from __future__ import annotations

import sys
from pathlib import Path


PROJECT_ROOT = Path(__file__).resolve().parents[1]
GENERATED_ROOT = PROJECT_ROOT / "output" / "python"
IMPLEMENTATION_ROOT = PROJECT_ROOT / "python"


def configure_generated_imports() -> Path:
    """Make generated components and local service contracts importable."""
    if not GENERATED_ROOT.is_dir():
        raise RuntimeError(
            "Generated Python sources are missing. Run the SPEADL generator first."
        )

    for path in (GENERATED_ROOT, IMPLEMENTATION_ROOT):
        spelling = str(path)
        if spelling not in sys.path:
            sys.path.insert(0, spelling)

    return GENERATED_ROOT
