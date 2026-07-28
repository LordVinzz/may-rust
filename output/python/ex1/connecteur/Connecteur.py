from __future__ import annotations
from abc import ABC, abstractmethod
from ex1.Start import *
_UNSET = object()

class Connecteur(ABC):

    class _Requires:

        def __init__(self, owner):
            self._owner = owner

        def demarreur1(self) -> Start:
            if self._owner._required_demarreur1 is _UNSET:
                raise RuntimeError('required service `demarreur1` is not bound')
            return self._owner._required_demarreur1

        def demarreur2(self) -> Start:
            if self._owner._required_demarreur2 is _UNSET:
                raise RuntimeError('required service `demarreur2` is not bound')
            return self._owner._required_demarreur2

        def demarreur3(self) -> Start:
            if self._owner._required_demarreur3 is _UNSET:
                raise RuntimeError('required service `demarreur3` is not bound')
            return self._owner._required_demarreur3

    class _Parts:

        def __init__(self, owner):
            self._owner = owner

    def __init__(self):
        self._required_demarreur1 = _UNSET
        self._required_demarreur2 = _UNSET
        self._required_demarreur3 = _UNSET
        self._provided_starter = _UNSET
        self._requires_view = self._Requires(self)
        self._parts_view = self._Parts(self)

    def requires(self) -> _Requires:
        return self._requires_view

    def parts(self) -> _Parts:
        return self._parts_view

    def _bind_demarreur1(self, service: Start):
        self._required_demarreur1 = service

    def _bind_demarreur2(self, service: Start):
        self._required_demarreur2 = service

    def _bind_demarreur3(self, service: Start):
        self._required_demarreur3 = service

    @abstractmethod
    def make_starter(self) -> Start:
        raise NotImplementedError('implement `make_starter` in a concrete component')

    def starter(self) -> Start:
        if self._provided_starter is _UNSET:
            self._provided_starter = self.make_starter()
        return self._provided_starter
