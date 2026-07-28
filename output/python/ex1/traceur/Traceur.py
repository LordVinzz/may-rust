from __future__ import annotations
from abc import ABC, abstractmethod
from ex1.Start import *
_UNSET = object()

class Traceur(ABC):

    class _Requires:

        def __init__(self, owner):
            self._owner = owner

        def starter(self) -> Start:
            if self._owner._required_starter is _UNSET:
                raise RuntimeError('required service `starter` is not bound')
            return self._owner._required_starter

    class _Parts:

        def __init__(self, owner):
            self._owner = owner

    def __init__(self):
        self._required_starter = _UNSET
        self._provided_demarreur = _UNSET
        self._requires_view = self._Requires(self)
        self._parts_view = self._Parts(self)

    def requires(self) -> _Requires:
        return self._requires_view

    def parts(self) -> _Parts:
        return self._parts_view

    def _bind_starter(self, service: Start):
        self._required_starter = service

    @abstractmethod
    def make_demarreur(self) -> Start:
        raise NotImplementedError('implement `make_demarreur` in a concrete component')

    def demarreur(self) -> Start:
        if self._provided_demarreur is _UNSET:
            self._provided_demarreur = self.make_demarreur()
        return self._provided_demarreur
