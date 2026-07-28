from __future__ import annotations
from abc import ABC, abstractmethod
from ex1.codec.Codec import *
from ex1.traceur.Traceur import *
from ex1.Start import *
_UNSET = object()

class Cypher(Traceur):

    class _Requires(Traceur._Requires):
        pass

    class _Parts(Traceur._Parts):

        def codeur(self) -> Codec[Start]:
            return self._owner._get_part_codeur()

        def decodeur(self) -> Codec[Start]:
            return self._owner._get_part_decodeur()

    def __init__(self):
        super().__init__()
        self._provided_demarreur = _UNSET
        self._part_codeur_cache = _UNSET
        self._part_decodeur_cache = _UNSET
        self._requires_view = self._Requires(self)
        self._parts_view = self._Parts(self)

    def requires(self) -> _Requires:
        return self._requires_view

    def parts(self) -> _Parts:
        return self._parts_view

    def make_demarreur(self) -> Start:
        return self.parts().decodeur().crypt()

    def demarreur(self) -> Start:
        if self._provided_demarreur is _UNSET:
            self._provided_demarreur = self.make_demarreur()
        return self._provided_demarreur

    @abstractmethod
    def make_codeur(self) -> Codec[Start]:
        raise NotImplementedError('implement `make_codeur` in a concrete component')

    def _get_part_codeur(self) -> Codec[Start]:
        if self._part_codeur_cache is _UNSET:
            part = self.make_codeur()
            self._part_codeur_cache = part
            part._bind_message(self.requires().starter())
        return self._part_codeur_cache

    @abstractmethod
    def make_decodeur(self) -> Codec[Start]:
        raise NotImplementedError('implement `make_decodeur` in a concrete component')

    def _get_part_decodeur(self) -> Codec[Start]:
        if self._part_decodeur_cache is _UNSET:
            part = self.make_decodeur()
            self._part_decodeur_cache = part
            part._bind_message(self.parts().codeur().crypt())
        return self._part_decodeur_cache
