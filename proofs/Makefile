ROCQ ?= opam exec -- rocq

.PHONY: all clean recheck

all: Grammar.vo

Grammar.vo: Grammar.v _CoqProject
	$(ROCQ) c -Q . MayRustProofs Grammar.v

clean:
	rm -f *.vo *.vos *.vok *.glob .*.aux

recheck:
	$(MAKE) clean
	$(MAKE) all
