from ex1.codec.Codec import *
from ex1.traceur.Traceur import *
from ex1.Start import *

class Cypher(Traceur):

    def __init__(self, starter: Start, codeur: Codec, decodeur: Codec):
        super().__init__(starter)
        self.codeur = codeur
        self.decodeur = decodeur
        self.codeur.message = self.starter
        self.decodeur.message = self.codeur.crypt
        return

    def demarreur(self) -> Start:
        return self.decodeur.crypt()
