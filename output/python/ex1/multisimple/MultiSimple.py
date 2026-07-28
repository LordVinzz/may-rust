from __future__ import annotations
from abc import ABC, abstractmethod
from ex1.simple.Simple import *
from ex1.client.Client import *
from ex1.connecteur.Connecteur import *
_UNSET = object()

class MultiSimple(ABC):

    class _Requires:

        def __init__(self, owner):
            self._owner = owner

    class _Parts:

        def __init__(self, owner):
            self._owner = owner

        def simple1(self) -> Simple:
            return self._owner._get_part_simple1()

        def simple2(self) -> Simple:
            return self._owner._get_part_simple2()

        def simple3(self) -> Simple:
            return self._owner._get_part_simple3()

        def client(self) -> Client:
            return self._owner._get_part_client()

        def connecteur(self) -> Connecteur:
            return self._owner._get_part_connecteur()

    def __init__(self):
        self._provided_service = _UNSET
        self._part_simple1_cache = _UNSET
        self._part_simple2_cache = _UNSET
        self._part_simple3_cache = _UNSET
        self._part_client_cache = _UNSET
        self._part_connecteur_cache = _UNSET
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
    def make_simple1(self) -> Simple:
        raise NotImplementedError('implement `make_simple1` in a concrete component')

    def _get_part_simple1(self) -> Simple:
        if self._part_simple1_cache is _UNSET:
            part = self.make_simple1()
            self._part_simple1_cache = part
        return self._part_simple1_cache

    @abstractmethod
    def make_simple2(self) -> Simple:
        raise NotImplementedError('implement `make_simple2` in a concrete component')

    def _get_part_simple2(self) -> Simple:
        if self._part_simple2_cache is _UNSET:
            part = self.make_simple2()
            self._part_simple2_cache = part
        return self._part_simple2_cache

    @abstractmethod
    def make_simple3(self) -> Simple:
        raise NotImplementedError('implement `make_simple3` in a concrete component')

    def _get_part_simple3(self) -> Simple:
        if self._part_simple3_cache is _UNSET:
            part = self.make_simple3()
            self._part_simple3_cache = part
        return self._part_simple3_cache

    @abstractmethod
    def make_client(self) -> Client:
        raise NotImplementedError('implement `make_client` in a concrete component')

    def _get_part_client(self) -> Client:
        if self._part_client_cache is _UNSET:
            part = self.make_client()
            self._part_client_cache = part
            part._bind_demarreur(self.parts().connecteur().starter())
        return self._part_client_cache

    @abstractmethod
    def make_connecteur(self) -> Connecteur:
        raise NotImplementedError('implement `make_connecteur` in a concrete component')

    def _get_part_connecteur(self) -> Connecteur:
        if self._part_connecteur_cache is _UNSET:
            part = self.make_connecteur()
            self._part_connecteur_cache = part
            part._bind_demarreur1(self.parts().simple1().starter())
            part._bind_demarreur2(self.parts().simple2().starter())
            part._bind_demarreur3(self.parts().simple3().starter())
        return self._part_connecteur_cache
