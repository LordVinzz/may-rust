from ex1.Start import *

class Client :
	def __init__(self, demarreur : Start):
		self.demarreur = demarreur
		return

	def letsgo(self) -> Runnable:
		return