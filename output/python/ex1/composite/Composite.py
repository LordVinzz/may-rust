from __future__ import annotations
from abc import ABC, abstractmethod
from ex1.simple.Simple import *
from ex1.client.Client import *
_UNSET = object()

class Composite(ABC):

    class _Requires:

        def __init__(self, owner):
            self._owner = owner

    class _Parts:

        def __init__(self, owner):
            self._owner = owner

        def simple(self) -> Simple:
            return self._owner._get_part_simple()

        def client(self) -> Client:
            return self._owner._get_part_client()

    def __init__(self):
        self._provided_service = _UNSET
        self._part_simple_cache = _UNSET
        self._part_client_cache = _UNSET
        self._requires_view = self._Requires(self)
        self._parts_view = self._Parts(self)

    def requires(self) -> _Requires:
        return self._requires_view

    def parts(self) -> _Parts:
        return self._parts_view

    def make_service(self) -> Runnable:
        return self.parts().client().letsgo()

    def service(self) -> Runnable:
        if self._provided_service is _UNSET:
            self._provided_service = self.make_service()
        return self._provided_service

    @abstractmethod
    def make_simple(self) -> Simple:
        raise NotImplementedError('implement `make_simple` in a concrete component')

    def _get_part_simple(self) -> Simple:
        if self._part_simple_cache is _UNSET:
            part = self.make_simple()
            self._part_simple_cache = part
        return self._part_simple_cache

    @abstractmethod
    def make_client(self) -> Client:
        raise NotImplementedError('implement `make_client` in a concrete component')

    def _get_part_client(self) -> Client:
        if self._part_client_cache is _UNSET:
            part = self.make_client()
            self._part_client_cache = part
            part._bind_demarreur(self.parts().simple().starter())
        return self._part_client_cache
