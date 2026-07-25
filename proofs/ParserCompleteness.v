From Stdlib Require Import Lists.List.
From MayRustProofs Require Import Grammar ParserEquivalence ParserDeterminism.

Import ListNotations.
Import MayRustGrammar.
Import ParserEquivalence.

(**
 * @brief Regroupe les définitions et les preuves de complétude du parseur.
 *)
Module ParserCompleteness.

(**
 * @brief Caractérise un flux de jetons valide par l'existence d'un témoin de la grammaire déclarative.
 * @param resolve Résolveur utilisé pour traiter les imports pendant le parsage.
 * @param input Flux de jetons à reconnaître.
 * @return Une proposition affirmant qu'un programme et un reste de flux peuvent être produits par la grammaire.
 *)
Definition valid_program_tokens
    (resolve : import_resolver) (input : list token) : Prop :=
  exists program rest, parses_namespace resolve input program rest.

(**
 * @brief Caractérise l'acceptation d'un flux de jetons par une exécution du parseur Rust.
 * @param run Modèle fonctionnel de l'exécution du parseur Rust.
 * @param resolve Résolveur utilisé pour traiter les imports.
 * @param input Flux de jetons soumis au parseur.
 * @return Une proposition affirmant que l'exécution accepte un programme avec un reste de flux.
 *)
Definition parser_accepts
    (run : rust_parser_runner)
    (resolve : import_resolver)
    (input : list token) : Prop :=
  exists program rest, run resolve input = ParserAccepted program rest.

(**
 * @brief Définit la complétude dans le sens Coq vers Rust : tout parsage grammatical est reproduit par le parseur.
 * @param run Modèle fonctionnel de l'exécution du parseur Rust.
 * @return Une proposition reliant chaque dérivation grammaticale au même AST et au même reste renvoyés par le parseur.
 *)
Definition parser_complete_for_grammar (run : rust_parser_runner) : Prop :=
  forall resolve input program rest,
    parses_namespace resolve input program rest ->
    run resolve input = ParserAccepted program rest.

(**
 * @brief Déduit la complétude du parseur à partir de son équivalence bidirectionnelle avec la relation Coq.
 * @param run Modèle fonctionnel de l'exécution du parseur Rust.
 * @return Si le parseur est équivalent à la relation Coq, alors il est complet pour la grammaire.
 *)
Theorem equivalence_implies_parser_completeness :
  forall run,
    parser_equivalent_to_rust run ->
    parser_complete_for_grammar run.
Proof.
  intros run Hequivalent resolve input program rest Hparse.
  apply (proj2 (Hequivalent resolve input program rest)).
  exact Hparse.
Qed.

(**
 * @brief Établit qu'un parseur complet accepte tout flux valide selon la grammaire.
 * @param run Modèle fonctionnel de l'exécution du parseur Rust.
 * @param resolve Résolveur utilisé pour les imports.
 * @param input Flux de jetons considéré.
 * @return L'acceptation du flux dès que la complétude et sa validité grammaticale sont établies.
 *)
Theorem complete_parser_accepts_every_valid_stream :
  forall run resolve input,
    parser_complete_for_grammar run ->
    valid_program_tokens resolve input ->
    parser_accepts run resolve input.
Proof.
  intros run resolve input Hcomplete [program [rest Hparse]].
  exists program, rest.
  apply Hcomplete.
  exact Hparse.
Qed.

(**
 * @brief Établit qu'un parseur équivalent à la relation Coq accepte tout flux grammaticalement valide.
 * @param run Modèle fonctionnel de l'exécution du parseur Rust.
 * @param resolve Résolveur utilisé pour les imports.
 * @param input Flux de jetons considéré.
 * @return L'acceptation du flux sous les hypothèses d'équivalence et de validité grammaticale.
 *)
Theorem equivalent_parser_accepts_every_valid_stream :
  forall run resolve input,
    parser_equivalent_to_rust run ->
    valid_program_tokens resolve input ->
    parser_accepts run resolve input.
Proof.
  intros run resolve input Hequivalent Hvalid.
  eapply complete_parser_accepts_every_valid_stream.
  - apply equivalence_implies_parser_completeness. exact Hequivalent.
  - exact Hvalid.
Qed.

(**
 * @brief Montre qu'un parseur complet renvoie exactement l'unique résultat décrit par la grammaire.
 * @param run Modèle fonctionnel de l'exécution du parseur Rust.
 * @param resolve Résolveur utilisé pour les imports.
 * @param input Flux de jetons analysé.
 * @param program AST produit par la relation grammaticale.
 * @param rest Reste produit par la relation grammaticale.
 * @param returned_program AST renvoyé par le parseur.
 * @param returned_rest Reste renvoyé par le parseur.
 * @return L'égalité des AST et des restes produits par les deux descriptions du parsage.
 *)
Theorem complete_parser_returns_the_unique_grammar_result :
  forall run resolve input program rest returned_program returned_rest,
    parser_complete_for_grammar run ->
    parses_namespace resolve input program rest ->
    run resolve input = ParserAccepted returned_program returned_rest ->
    program = returned_program /\ rest = returned_rest.
Proof.
  intros run resolve input program rest returned_program returned_rest
         Hcomplete Hparse Hrun.
  pose proof (Hcomplete resolve input program rest Hparse) as Hexpected.
  rewrite Hrun in Hexpected.
  inversion Hexpected. auto.
Qed.

End ParserCompleteness.
