From Stdlib Require Import Lists.List.
From MayRustProofs Require Import Grammar ParserDeterminism.

Import ListNotations.
Import MayRustGrammar.

(**
 * @brief Regroupe la spécification du pont d'équivalence entre les parseurs Rust et Coq.
 *)
Module ParserEquivalence.

(**
 * @brief Représente le résultat observable d'une exécution du parseur Rust.
 * @return Soit un AST accepté avec les tokens restants, soit un rejet.
 *)
Inductive parser_outcome : Type :=
| ParserAccepted : ast -> list token -> parser_outcome
| ParserRejected : parser_outcome.

(**
 * @brief Décrit la signature abstraite permettant d'exécuter le parseur Rust.
 * @return Une fonction prenant un résolveur et des tokens, puis produisant un résultat de parsing.
 *)
Definition rust_parser_runner : Type :=
  import_resolver -> list token -> parser_outcome.

(**
 * @brief Spécifie l'équivalence exacte entre l'exécution Rust et la relation Coq du namespace.
 * @param run Interprétation ou traduction exécutable de la fonction Rust [Parser::namespace].
 * @return Une équivalence bidirectionnelle entre acceptation Rust et dérivation Coq.
 *)
Definition parser_equivalent_to_rust (run : rust_parser_runner) : Prop :=
  forall resolve input program rest,
    run resolve input = ParserAccepted program rest <->
    parses_namespace resolve input program rest.

(**
 * @brief Spécifie la correction du parseur Rust relativement à la relation Coq.
 * @param run Exécuteur abstrait du parseur Rust.
 * @return Une proposition transformant toute acceptation Rust en dérivation Coq.
 *)
Definition parser_rust_to_coq (run : rust_parser_runner) : Prop :=
  forall resolve input program rest,
    run resolve input = ParserAccepted program rest ->
    parses_namespace resolve input program rest.

(**
 * @brief Spécifie la complétude du parseur Rust relativement à la relation Coq.
 * @param run Exécuteur abstrait du parseur Rust.
 * @return Une proposition transformant toute dérivation Coq en acceptation Rust.
 *)
Definition parser_coq_to_rust (run : rust_parser_runner) : Prop :=
  forall resolve input program rest,
    parses_namespace resolve input program rest ->
    run resolve input = ParserAccepted program rest.

(**
 * @brief Décompose l'équivalence du parseur en correction et complétude.
 * @param run Exécuteur abstrait du parseur Rust.
 * @return L'équivalence entre le pont bidirectionnel et la conjonction de ses deux directions.
 *)
Theorem parser_equivalence_is_bidirectional :
  forall run,
    parser_equivalent_to_rust run <->
    parser_rust_to_coq run /\ parser_coq_to_rust run.
Proof.
  intros run. split.
  - intros Hequivalent. split.
    + intros resolve input program rest Hrun.
      apply (proj1 (Hequivalent resolve input program rest)).
      exact Hrun.
    + intros resolve input program rest Hparse.
      apply (proj2 (Hequivalent resolve input program rest)).
      exact Hparse.
  - intros [Hsound Hcomplete] resolve input program rest.
    split; auto.
Qed.

(**
 * @brief Prouve qu'un parseur Rust équivalent construit uniquement des AST bien formés.
 * @param run Exécuteur abstrait du parseur Rust.
 * @param resolve Résolveur d'import fourni à l'exécution.
 * @param input Flux de tokens soumis au parseur.
 * @param program AST accepté par le parseur.
 * @param rest Flux restant après l'acceptation.
 * @return La conformité grammaticale et les deux propriétés de bonne formation de l'AST.
 *)
Theorem equivalent_rust_parser_constructs_well_formed_ast :
  forall run resolve input program rest,
    parser_equivalent_to_rust run ->
    run resolve input = ParserAccepted program rest ->
    grammar_ast program /\ ast_wf program /\ program_wf program.
Proof.
  intros run resolve input program rest Hequivalent Hrun.
  apply (parses_namespace_sound resolve input program rest).
  apply (proj1 (Hequivalent resolve input program rest)).
  exact Hrun.
Qed.

(**
 * @brief Prouve l'unicité de toute acceptation réussie d'un parseur Rust équivalent.
 * @param run Exécuteur abstrait du parseur Rust.
 * @param resolve Résolveur d'import fourni aux exécutions.
 * @param input Flux de tokens commun aux deux exécutions.
 * @param program1 Premier AST accepté avec son reste [rest1].
 * @param program2 Second AST accepté avec son reste [rest2].
 * @return L'égalité des AST acceptés et de leurs flux restants.
 *)
Theorem equivalent_rust_parser_has_unique_success :
  forall run resolve input program1 rest1 program2 rest2,
    parser_equivalent_to_rust run ->
    run resolve input = ParserAccepted program1 rest1 ->
    run resolve input = ParserAccepted program2 rest2 ->
    program1 = program2 /\ rest1 = rest2.
Proof.
  intros run resolve input program1 rest1 program2 rest2
         Hequivalent Hrun1 Hrun2.
  eapply ParserDeterminism.parses_namespace_deterministic.
  - apply (proj1 (Hequivalent resolve input program1 rest1)). exact Hrun1.
  - apply (proj1 (Hequivalent resolve input program2 rest2)). exact Hrun2.
Qed.

End ParserEquivalence.
