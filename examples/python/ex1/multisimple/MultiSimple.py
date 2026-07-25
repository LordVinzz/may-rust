from ex1.simple.Simple import *
from ex1.client.Client import *
from ex1.connecteur.Connecteur import *

class MultiSimple:

    def __init__(self, simple1: Simple, simple2: Simple, simple3: Simple, client: Client, connecteur: Connecteur):
        self.simple1 = simple1
        self.simple2 = simple2
        self.simple3 = simple3
        self.client = client
        self.connecteur = connecteur
        self.client.demarreur = self.connecteur.starter
        self.connecteur.demarreur1 = self.simple1.starter
        self.connecteur.demarreur2 = self.simple2.starter
        self.connecteur.demarreur3 = self.simple3.starter
        return

    def service(self) -> Runnable:
        return self.client.letsgo()
