from __future__ import annotations

from ex1.Start import Start
from ex1.composite.Composite import Composite
from ex1.comptraceur.CompTraceur import CompTraceur
from ex1.cypher.Cypher import Cypher
from ex1.multiclient.MultiClient import MultiClient
from ex1.multisimple.MultiSimple import MultiSimple

from .components import (
    ClientImpl,
    CodecImpl,
    ConnecteurImpl,
    SimpleImpl,
    TraceurImpl,
)


class CompositeImpl(Composite):
    def make_simple(self) -> SimpleImpl:
        return SimpleImpl("composite.simple")

    def make_client(self) -> ClientImpl:
        return ClientImpl("composite.client")


class CompTraceurImpl(CompTraceur):
    def make_simple(self) -> SimpleImpl:
        return SimpleImpl("trace.simple")

    def make_traceur(self) -> TraceurImpl:
        return TraceurImpl()

    def make_client(self) -> ClientImpl:
        return ClientImpl("trace.client")


class CypherImpl(Cypher):
    def __init__(self, starter: Start) -> None:
        super().__init__()
        self._bind_starter(starter)

    def make_codeur(self) -> CodecImpl[Start]:
        return CodecImpl()

    def make_decodeur(self) -> CodecImpl[Start]:
        return CodecImpl()


class MultiClientImpl(MultiClient):
    def make_simple(self) -> SimpleImpl:
        return SimpleImpl("multi-client.simple")

    def make_client1(self) -> ClientImpl:
        return ClientImpl("client-1")

    def make_client2(self) -> ClientImpl:
        return ClientImpl("client-2")

    def make_client3(self) -> ClientImpl:
        return ClientImpl("client-3")


class MultiSimpleImpl(MultiSimple):
    def make_simple1(self) -> SimpleImpl:
        return SimpleImpl("simple-1")

    def make_simple2(self) -> SimpleImpl:
        return SimpleImpl("simple-2")

    def make_simple3(self) -> SimpleImpl:
        return SimpleImpl("simple-3")

    def make_client(self) -> ClientImpl:
        return ClientImpl("multi-simple.client")

    def make_connecteur(self) -> ConnecteurImpl:
        return ConnecteurImpl()
