from __future__ import annotations
from abc import ABC, abstractmethod
from ex1.Start import *
_UNSET = object()

class Simple(ABC):

    class _Requires:

        def __init__(self, owner):
            self._owner = owner

    class _Parts:

        def __init__(self, owner):
            self._owner = owner

    def __init__(self):
        self._provided_starter = _UNSET
        self._requires_view = self._Requires(self)
        self._parts_view = self._Parts(self)

    def requires(self) -> _Requires:
        return self._requires_view

    def parts(self) -> _Parts:
        return self._parts_view

    @abstractmethod
    def make_starter(self) -> Start:
        raise NotImplementedError('implement `make_starter` in a concrete component')

    def starter(self) -> Start:
        if self._provided_starter is _UNSET:
            self._provided_starter = self.make_starter()
        return self._provided_starter
