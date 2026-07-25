from ex1.simple.Simple import *
from ex1.client.Client import *
from ex1.traceur.Traceur import *

class CompTraceur:

    def __init__(self, simple: Simple, traceur: Traceur, client: Client):
        self.simple = simple
        self.traceur = traceur
        self.client = client
        self.traceur.starter = self.simple.starter
        self.client.demarreur = self.traceur.demarreur
        return

    def service(self) -> Runnable:
        return self.client.letsgo()
