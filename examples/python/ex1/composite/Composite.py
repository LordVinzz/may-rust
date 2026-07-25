from ex1.simple.Simple import *
from ex1.client.Client import *

class Composite:

    def __init__(self, simple: Simple, client: Client):
        self.simple = simple
        self.client = client
        self.client.demarreur = self.simple.starter
        return

    def service(self) -> Runnable:
        return self.client.letsgo()
