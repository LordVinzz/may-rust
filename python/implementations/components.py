from __future__ import annotations

from typing import Generic, TypeVar

from ex1.Start import FanOutStart, Runnable, Start, TracingStart
from ex1.client.Client import Client
from ex1.codec.Codec import Codec
from ex1.connecteur.Connecteur import Connecteur
from ex1.simple.Simple import Simple
from ex1.traceur.Traceur import Traceur


Service = TypeVar("Service")


class SimpleImpl(Simple):
    def __init__(self, name: str = "simple") -> None:
        super().__init__()
        self.name = name
        self.created_services = 0

    def make_starter(self) -> Start:
        self.created_services += 1
        return Start(self.name)


class ClientImpl(Client):
    def __init__(self, name: str = "client") -> None:
        super().__init__()
        self.name = name

    def make_letsgo(self) -> Runnable:
        def letsgo() -> str:
            return self.requires().demarreur().go()

        return letsgo


class CodecImpl(Codec[Service], Generic[Service]):
    """Identity codec that makes data flow through bindings observable."""

    def make_crypt(self) -> Service:
        return self.requires().message()


class TraceurImpl(Traceur):
    def __init__(self) -> None:
        super().__init__()
        self.events: list[str] = []

    def make_demarreur(self) -> Start:
        return TracingStart(self.requires().starter(), self.events)


class ConnecteurImpl(Connecteur):
    def make_starter(self) -> Start:
        return FanOutStart(
            [
                self.requires().demarreur1(),
                self.requires().demarreur2(),
                self.requires().demarreur3(),
            ]
        )
