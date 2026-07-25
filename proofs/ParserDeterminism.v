From Stdlib Require Import Lists.List.
From Stdlib Require Import Program.Equality.
From MayRustProofs Require Import Grammar.

Import ListNotations.
Import MayRustGrammar.

(**
 * @brief Regroupe les preuves de déterminisme des relations de parsing.
 *)
Module ParserDeterminism.

(**
 * @brief Caractérise une relation de parsing dont la sortie et le reste sont uniques.
 * @param relation Relation reliant une entrée à une sortie et à une entrée restante.
 * @return Une proposition affirmant l'unicité de la sortie et du reste pour chaque entrée.
 *)
Definition deterministic {Input Output Rest : Type}
    (relation : Input -> Output -> Rest -> Prop) : Prop :=
  forall input output1 rest1 output2 rest2,
    relation input output1 rest1 ->
    relation input output2 rest2 ->
    output1 = output2 /\ rest1 = rest2.

(**
 * @brief Établit le déterminisme du parsing récursif de la fin d'un chemin.
 * @param acc Préfixe de chemin déjà accumulé.
 * @param input Flux de tokens analysé.
 * @param path1 Premier chemin obtenu avec son reste [rest1].
 * @param path2 Second chemin obtenu avec son reste [rest2].
 * @return L'égalité des deux chemins produits et de leurs flux restants.
 *)
Lemma parses_path_tail_deterministic :
  forall acc input path1 rest1 path2 rest2,
    parses_path_tail acc input path1 rest1 ->
    parses_path_tail acc input path2 rest2 ->
    path1 = path2 /\ rest1 = rest2.
Proof.
  intros acc input path1 rest1 path2 rest2 Hleft.
  generalize dependent rest2.
  generalize dependent path2.
  induction Hleft; intros path2 rest2 Hright;
    inversion Hright; subst; simpl in *; try contradiction; eauto.
Qed.

(**
 * @brief Établit le déterminisme du parsing d'un chemin complet.
 * @return La propriété [deterministic] appliquée à [parses_path].
 *)
Theorem parses_path_deterministic : deterministic parses_path.
Proof.
  intros input path1 rest1 path2 rest2 Hleft Hright.
  dependent destruction Hleft.
  dependent destruction Hright.
  eapply parses_path_tail_deterministic; eassumption.
Qed.

(**
 * @brief Établit le déterminisme du parsing d'une déclaration générique optionnelle.
 * @return La propriété [deterministic] appliquée à [parses_generic].
 *)
Theorem parses_generic_deterministic : deterministic parses_generic.
Proof.
  intros input generic1 rest1 generic2 rest2 Hleft Hright.
  dependent destruction Hleft; dependent destruction Hright;
    simpl in *; try contradiction; auto.
Qed.

(**
 * @brief Établit le déterminisme du parsing d'une spécialisation optionnelle.
 * @param resolve Résolveur d'import utilisé pendant l'analyse de la spécialisation.
 * @return La propriété [deterministic] appliquée à [parses_specializes resolve].
 *)
Theorem parses_specializes_deterministic :
  forall resolve, deterministic (parses_specializes resolve).
Proof.
  intros resolve input specializes1 rest1 specializes2 rest2 Hleft Hright.
  dependent destruction Hleft; dependent destruction Hright;
    simpl in *; try contradiction; auto.
Qed.

(**
 * @brief Établit le déterminisme du parsing d'une implémentation.
 * @return La propriété [deterministic] appliquée à [parses_implementation].
 *)
Theorem parses_implementation_deterministic :
  deterministic parses_implementation.
Proof.
  intros input implementation1 rest1 implementation2 rest2 Hleft Hright.
  dependent destruction Hleft; dependent destruction Hright;
    simpl in *; try contradiction; auto.
Qed.

(**
 * @brief Établit le déterminisme du parsing d'une séquence de liaisons.
 * @return La propriété [deterministic] appliquée à [parses_binds].
 *)
Theorem parses_binds_deterministic : deterministic parses_binds.
Proof.
  intros input binds1 rest1 binds2 rest2 Hleft.
  generalize dependent rest2.
  generalize dependent binds2.
  induction Hleft; intros binds2 rest2 Hright;
    dependent destruction Hright; simpl in *; try contradiction.
  - auto.
  - destruct (IHHleft _ _ Hright) as [-> ->]. auto.
  - destruct (IHHleft _ _ Hright) as [-> ->]. auto.
Qed.

(**
 * @brief Établit le déterminisme du parsing d'une séquence de parties.
 * @return La propriété [deterministic] appliquée à [parses_parts].
 *)
Theorem parses_parts_deterministic : deterministic parses_parts.
Proof.
  intros input parts1 rest1 parts2 rest2 Hleft.
  generalize dependent rest2.
  generalize dependent parts2.
  induction Hleft; intros parts2 rest2 Hright;
    dependent destruction Hright; simpl in *; try contradiction.
  - auto.
  - destruct (parses_generic_deterministic
                _ _ _ _ _ H H1) as [-> Hbody].
    inversion Hbody; subst.
    destruct (parses_binds_deterministic
                _ _ _ _ _ H0 H2) as [-> HafterPart].
    inversion HafterPart; subst.
    destruct (IHHleft _ _ Hright) as [-> ->].
    auto.
Qed.

(**
 * @brief Montre qu'une entrée consommée comme fourniture ne peut pas commencer par un token d'arrêt.
 * @param input Flux de tokens analysé.
 * @param entries Entrées de fourniture produites par le parsing.
 * @param rest Flux restant après le parsing.
 * @return Une contradiction lorsque [provides_can_stop input] est également vérifié.
 *)
Lemma parsed_provide_entry_cannot_be_a_stop_token :
  forall input entries rest,
    parses_provide_entries input entries rest ->
    provides_can_stop input ->
    False.
Proof.
  intros input entries rest Hparse Hstop.
  inversion Hparse; subst; simpl in Hstop; contradiction.
Qed.

(**
 * @brief Établit le déterminisme du parsing des entrées d'une section de fournitures.
 * @return La propriété [deterministic] appliquée à [parses_provide_entries].
 *)
Theorem parses_provide_entries_deterministic :
  deterministic parses_provide_entries.
Proof.
  intros input entries1 rest1 entries2 rest2 Hleft.
  generalize dependent rest2.
  generalize dependent entries2.
  induction Hleft; intros entries2 rest2 Hright;
    dependent destruction Hright.
  - destruct (parses_implementation_deterministic
                _ _ _ _ _ H H1) as [-> ->].
    auto.
  - destruct (parses_implementation_deterministic
                _ _ _ _ _ H H1) as [_ Hrest].
    inversion Hrest; subst.
    exfalso.
    eapply parsed_provide_entry_cannot_be_a_stop_token; eassumption.
  - destruct (parses_implementation_deterministic
                _ _ _ _ _ H H0) as [_ Hrest].
    inversion Hrest; subst.
    exfalso.
    eapply parsed_provide_entry_cannot_be_a_stop_token; eassumption.
  - destruct (parses_implementation_deterministic
                _ _ _ _ _ H H0) as [-> Hafter].
    inversion Hafter; subst.
    destruct (IHHleft _ _ Hright) as [-> ->].
    auto.
Qed.

(**
 * @brief Établit le déterminisme du parsing d'une section de fournitures.
 * @return La propriété [deterministic] appliquée à [parses_provides].
 *)
Theorem parses_provides_deterministic : deterministic parses_provides.
Proof.
  intros input nodes1 rest1 nodes2 rest2 Hleft Hright.
  dependent destruction Hleft.
  dependent destruction Hright.
  destruct (parses_provide_entries_deterministic
              _ _ _ _ _ H H1) as [-> HafterProvides].
  inversion HafterProvides; subst.
  destruct (parses_parts_deterministic
              _ _ _ _ _ H0 H2) as [-> ->].
  auto.
Qed.

(**
 * @brief Établit le déterminisme du parsing d'une séquence d'exigences.
 * @return La propriété [deterministic] appliquée à [parses_requires].
 *)
Theorem parses_requires_deterministic : deterministic parses_requires.
Proof.
  intros input nodes1 rest1 nodes2 rest2 Hleft.
  generalize dependent rest2.
  generalize dependent nodes2.
  induction Hleft; intros nodes2 rest2 Hright;
    dependent destruction Hright; simpl in *; try contradiction.
  - eapply parses_provides_deterministic; eassumption.
  - destruct (IHHleft _ _ Hright) as [-> ->]. auto.
Qed.

(**
 * @brief Établit le déterminisme du parsing d'un composant.
 * @param resolve Résolveur d'import consulté pendant l'analyse du composant.
 * @return La propriété [deterministic] appliquée à [parses_component resolve].
 *)
Theorem parses_component_deterministic :
  forall resolve, deterministic (parses_component resolve).
Proof.
  intros resolve input component1 rest1 component2 rest2 Hleft Hright.
  dependent destruction Hleft.
  dependent destruction Hright.
  destruct (parses_specializes_deterministic resolve
              _ _ _ _ _ H H2) as [-> HafterSpecializes].
  inversion HafterSpecializes; subst.
  destruct (parses_generic_deterministic
              _ _ _ _ _ H0 H3) as [-> Hbody].
  inversion Hbody; subst.
  destruct (parses_requires_deterministic
              _ _ _ _ _ H1 H4) as [-> HafterComponent].
  inversion HafterComponent; subst.
  auto.
Qed.

(**
 * @brief Établit le déterminisme du parsing de la liste des imports.
 * @return La propriété [deterministic] appliquée à [parses_imports].
 *)
Theorem parses_imports_deterministic : deterministic parses_imports.
Proof.
  intros input imports1 rest1 imports2 rest2 Hleft.
  generalize dependent rest2.
  generalize dependent imports2.
  induction Hleft; intros imports2 rest2 Hright;
    dependent destruction Hright; simpl in *; try contradiction.
  - auto.
  - destruct (parses_path_deterministic
                _ _ _ _ _ H H0) as [-> HafterPath].
    inversion HafterPath; subst.
    destruct (IHHleft _ _ Hright) as [-> ->].
    auto.
Qed.

(**
 * @brief Établit le déterminisme du parsing d'un espace de noms complet.
 * @param resolve Résolveur utilisé pour rechercher les imports du programme.
 * @return La propriété [deterministic] appliquée à [parses_namespace resolve].
 *)
Theorem parses_namespace_deterministic :
  forall resolve, deterministic (parses_namespace resolve).
Proof.
  intros resolve input program1 rest1 program2 rest2 Hleft Hright.
  dependent destruction Hleft.
  dependent destruction Hright.
  destruct (parses_imports_deterministic
              _ _ _ _ _ H H2) as [Himports HafterImports].
  subst imports0.
  inversion HafterImports; subst after_imports0.
  destruct (parses_path_deterministic
              _ _ _ _ _ H0 H3) as [Hpath HafterPath].
  subst path0.
  inversion HafterPath; subst after_path0.
  destruct (parses_component_deterministic
              (fun name => search_import resolve (import_paths imports) name)
              _ _ _ _ _ H1 H4) as [Hcomponent HafterNamespace].
  subst component0.
  inversion HafterNamespace; subst rest0.
  split; reflexivity.
Qed.

End ParserDeterminism.
