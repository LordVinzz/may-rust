from __future__ import annotations
from abc import ABC, abstractmethod
from typing import Generic, TypeVar
_UNSET = object()
Service = TypeVar('Service')

class Codec(Generic[Service], ABC):

    class _Requires:

        def __init__(self, owner):
            self._owner = owner

        def message(self) -> Service:
            if self._owner._required_message is _UNSET:
                raise RuntimeError('required service `message` is not bound')
            return self._owner._required_message

    class _Parts:

        def __init__(self, owner):
            self._owner = owner

    def __init__(self):
        self._required_message = _UNSET
        self._provided_crypt = _UNSET
        self._requires_view = self._Requires(self)
        self._parts_view = self._Parts(self)

    def requires(self) -> _Requires:
        return self._requires_view

    def parts(self) -> _Parts:
        return self._parts_view

    def _bind_message(self, service: Service):
        self._required_message = service

    @abstractmethod
    def make_crypt(self) -> Service:
        raise NotImplementedError('implement `make_crypt` in a concrete component')

    def crypt(self) -> Service:
        if self._provided_crypt is _UNSET:
            self._provided_crypt = self.make_crypt()
        return self._provided_crypt
