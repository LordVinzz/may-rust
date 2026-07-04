from ex1.simple.Simple import *
from ex1.client.Client import *

class Composite :
	def __init__(self):
		self.simple = Simple()
		self.demarreur = Client(self.simple.starter)
		return

	def service(self) -> Runnable:
		return self.client.letsgo()
