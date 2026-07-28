from __future__ import annotations

from collections.abc import Callable
from typing import TypeAlias


Runnable: TypeAlias = Callable[[], object]


class Start:
    """Small Python service contract corresponding to the Java Start service."""

    def __init__(self, name: str = "start") -> None:
        self.name = name
        self.calls = 0

    def go(self) -> str:
        self.calls += 1
        return f"{self.name}: call {self.calls}"


class TracingStart(Start):
    """Start decorator used by TraceurImpl."""

    def __init__(self, target: Start, events: list[str]) -> None:
        super().__init__(f"trace({target.name})")
        self.target = target
        self.events = events

    def go(self) -> str:
        self.calls += 1
        self.events.append(f"before {self.target.name}")
        result = self.target.go()
        self.events.append(f"after {self.target.name}")
        return result


class FanOutStart(Start):
    """Start service that invokes several required Start services."""

    def __init__(self, targets: list[Start]) -> None:
        super().__init__("fan-out")
        self.targets = targets

    def go(self) -> list[str]:
        self.calls += 1
        return [target.go() for target in self.targets]
