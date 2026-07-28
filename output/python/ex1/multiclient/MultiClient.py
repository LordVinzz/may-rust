from __future__ import annotations
from abc import ABC, abstractmethod
from ex1.simple.Simple import *
from ex1.client.Client import *
_UNSET = object()

class MultiClient(ABC):

    class _Requires:

        def __init__(self, owner):
            self._owner = owner

    class _Parts:

        def __init__(self, owner):
            self._owner = owner

        def simple(self) -> Simple:
            return self._owner._get_part_simple()

        def client1(self) -> Client:
            return self._owner._get_part_client1()

        def client2(self) -> Client:
            return self._owner._get_part_client2()

        def client3(self) -> Client:
            return self._owner._get_part_client3()

    def __init__(self):
        self._provided_service1 = _UNSET
        self._provided_service2 = _UNSET
        self._provided_service3 = _UNSET
        self._part_simple_cache = _UNSET
        self._part_client1_cache = _UNSET
        self._part_client2_cache = _UNSET
        self._part_client3_cache = _UNSET
        self._requires_view = self._Requires(self)
        self._parts_view = self._Parts(self)

    def requires(self) -> _Requires:
        return self._requires_view

    def parts(self) -> _Parts:
        return self._parts_view

    def make_service1(self) -> Runnable:
        return self.parts().client1().letsgo()

    def service1(self) -> Runnable:
        if self._provided_service1 is _UNSET:
            self._provided_service1 = self.make_service1()
        return self._provided_service1

    def make_service2(self) -> Runnable:
        return self.parts().client2().letsgo()

    def service2(self) -> Runnable:
        if self._provided_service2 is _UNSET:
            self._provided_service2 = self.make_service2()
        return self._provided_service2

    def make_service3(self) -> Runnable:
        return self.parts().client3().letsgo()

    def service3(self) -> Runnable:
        if self._provided_service3 is _UNSET:
            self._provided_service3 = self.make_service3()
        return self._provided_service3

    @abstractmethod
    def make_simple(self) -> Simple:
        raise NotImplementedError('implement `make_simple` in a concrete component')

    def _get_part_simple(self) -> Simple:
        if self._part_simple_cache is _UNSET:
            part = self.make_simple()
            self._part_simple_cache = part
        return self._part_simple_cache

    @abstractmethod
    def make_client1(self) -> Client:
        raise NotImplementedError('implement `make_client1` in a concrete component')

    def _get_part_client1(self) -> Client:
        if self._part_client1_cache is _UNSET:
            part = self.make_client1()
            self._part_client1_cache = part
            part._bind_demarreur(self.parts().simple().starter())
        return self._part_client1_cache

    @abstractmethod
    def make_client2(self) -> Client:
        raise NotImplementedError('implement `make_client2` in a concrete component')

    def _get_part_client2(self) -> Client:
        if self._part_client2_cache is _UNSET:
            part = self.make_client2()
            self._part_client2_cache = part
            part._bind_demarreur(self.parts().simple().starter())
        return self._part_client2_cache

    @abstractmethod
    def make_client3(self) -> Client:
        raise NotImplementedError('implement `make_client3` in a concrete component')

    def _get_part_client3(self) -> Client:
        if self._part_client3_cache is _UNSET:
            part = self.make_client3()
            self._part_client3_cache = part
            part._bind_demarreur(self.parts().simple().starter())
        return self._part_client3_cache
