from ex1.simple.Simple import *
from ex1.client.Client import *

class MultiClient:

    def __init__(self, simple: Simple, client1: Client, client2: Client, client3: Client):
        self.simple = simple
        self.client1 = client1
        self.client2 = client2
        self.client3 = client3
        self.client1.demarreur = self.simple.starter
        self.client2.demarreur = self.simple.starter
        self.client3.demarreur = self.simple.starter
        return

    def service1(self) -> Runnable:
        return self.client1.letsgo()

    def service2(self) -> Runnable:
        return self.client2.letsgo()

    def service3(self) -> Runnable:
        return self.client3.letsgo()
