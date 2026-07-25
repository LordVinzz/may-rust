ROCQ ?= opam exec -- rocq
PROOFS := Grammar.vo \
	LexerCorrectness.vo \
	ParserDeterminism.vo \
	ParserEquivalence.vo \
	ParserCompleteness.vo \
	ImportResolution.vo

.PHONY: all clean recheck

all: $(PROOFS)

Grammar.vo: Grammar.v _CoqProject
	$(ROCQ) c -Q . MayRustProofs Grammar.v

LexerCorrectness.vo: LexerCorrectness.v Grammar.vo _CoqProject
	$(ROCQ) c -Q . MayRustProofs LexerCorrectness.v

ParserDeterminism.vo: ParserDeterminism.v Grammar.vo _CoqProject
	$(ROCQ) c -Q . MayRustProofs ParserDeterminism.v

ParserEquivalence.vo: ParserEquivalence.v Grammar.vo ParserDeterminism.vo _CoqProject
	$(ROCQ) c -Q . MayRustProofs ParserEquivalence.v

ParserCompleteness.vo: ParserCompleteness.v Grammar.vo ParserDeterminism.vo ParserEquivalence.vo _CoqProject
	$(ROCQ) c -Q . MayRustProofs ParserCompleteness.v

ImportResolution.vo: ImportResolution.v Grammar.vo _CoqProject
	$(ROCQ) c -Q . MayRustProofs ImportResolution.v

clean:
	rm -f *.vo *.vos *.vok *.glob .*.aux .lia.cache

recheck:
	$(MAKE) clean
	$(MAKE) all
