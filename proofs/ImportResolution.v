From Stdlib Require Import Lists.List.
From Stdlib Require Import Strings.String.
From MayRustProofs Require Import Grammar.

Import ListNotations.
Import MayRustGrammar.

(**
 * @brief Regroupe le modèle pur de résolution des imports et ses propriétés de correction.
 *)
Module ImportResolution.

(**
 * @brief Représente un chemin physique comme une suite de composants textuels.
 * @return Le type abstrait des chemins manipulés par le modèle de résolution.
 *)
Definition physical_path : Type := list string.

(**
 * @brief Construit le nom de fichier SPEADL associé à un nom logique.
 * @param name Nom logique auquel ajouter l'extension.
 * @return Le nom suivi de l'extension « .speadl ».
 *)
Definition speadl_file_name (name : string) : string :=
  String.append name ".speadl"%string.

(**
 * @brief Ajoute l'extension SPEADL au dernier composant d'un chemin.
 * @param path Chemin logique à convertir en chemin de fichier.
 * @return Le chemin dont seul le dernier composant porte l'extension SPEADL.
 *)
Fixpoint add_speadl_extension_to_last (path : physical_path)
    : physical_path :=
  match path with
  | [] => []
  | [name] => [speadl_file_name name]
  | directory :: rest => directory :: add_speadl_extension_to_last rest
  end.

(**
 * @brief Construit le candidat situé dans le même répertoire que le fichier source.
 * @param source_directory Répertoire contenant le fichier source.
 * @param import Chemin logique de l'import demandé.
 * @return Le chemin candidat, ou None lorsque le chemin d'import est vide.
 *)
Definition same_directory_candidate
    (source_directory import : physical_path) : option physical_path :=
  match rev import with
  | [] => None
  | name :: _ => Some (source_directory ++ [speadl_file_name name])
  end.

(**
 * @brief Énumère les répertoires ancêtres à partir d'un chemin stocké en ordre inverse.
 * @param reversed_directory Composants du répertoire ordonnés du plus profond au plus haut.
 * @return La liste des répertoires depuis le répertoire initial jusqu'à la racine.
 *)
Fixpoint ancestors_of_reversed (reversed_directory : physical_path)
    : list physical_path :=
  match reversed_directory with
  | [] => [[]]
  | _ :: parent => rev reversed_directory :: ancestors_of_reversed parent
  end.

(**
 * @brief Énumère un répertoire et chacun de ses ancêtres.
 * @param directory Répertoire de départ dans son ordre habituel.
 * @return La liste des répertoires candidats jusqu'à la racine.
 *)
Definition ancestors (directory : physical_path) : list physical_path :=
  ancestors_of_reversed (rev directory).

(**
 * @brief Construit un chemin candidat qualifié sous un répertoire ancêtre.
 * @param ancestor Répertoire ancêtre servant de base.
 * @param import Chemin logique complet de l'import.
 * @return La concaténation du répertoire et du chemin d'import terminé par l'extension SPEADL.
 *)
Definition qualified_candidate
    (ancestor import : physical_path) : physical_path :=
  ancestor ++ add_speadl_extension_to_last import.

(**
 * @brief Énumère les chemins candidats dans l'ordre de recherche de resolve_import_path.
 * @param source_directory Répertoire contenant le fichier source.
 * @param import Chemin logique de l'import demandé.
 * @return Le candidat local, puis chaque ancêtre joint au chemin d'import complet.
 *)
Definition resolution_candidates
    (source_directory import : physical_path) : list physical_path :=
  match same_directory_candidate source_directory import with
  | None => []
  | Some same_directory =>
      same_directory ::
      map (fun ancestor => qualified_candidate ancestor import)
          (ancestors source_directory)
  end.

(**
 * @brief Sélectionne le premier chemin désigné comme fichier dans une liste de candidats.
 * @param is_file Prédicat booléen indiquant si un chemin correspond à un fichier.
 * @param candidates Chemins candidats examinés dans leur ordre de priorité.
 * @return Le premier chemin existant, ou None si aucun candidat n'existe.
 *)
Fixpoint first_existing
    (is_file : physical_path -> bool)
    (candidates : list physical_path) : option physical_path :=
  match candidates with
  | [] => None
  | candidate :: rest =>
      if is_file candidate
      then Some candidate
      else first_existing is_file rest
  end.

(**
 * @brief Décrit les opérations du système de fichiers nécessaires à la résolution pure des imports.
 * @param model_is_file Teste si un chemin désigne un fichier.
 * @param model_read_file Lit éventuellement le contenu textuel d'un chemin.
 * @param model_parse_file Parse éventuellement le contenu lu en AST.
 *)
Record filesystem_model : Type := {
  model_is_file : physical_path -> bool;
  model_read_file : physical_path -> option string;
  model_parse_file : physical_path -> string -> option ast
}.

(**
 * @brief Résout un import en recherchant, lisant puis parsant son premier fichier candidat existant.
 * @param filesystem Modèle des opérations de système de fichiers.
 * @param source_directory Répertoire contenant le fichier source.
 * @param import Chemin logique de l'import demandé.
 * @return L'AST résolu, ou None si la recherche, la lecture ou le parsage échoue.
 *)
Definition resolve_import_from_filesystem
    (filesystem : filesystem_model)
    (source_directory import : physical_path) : option ast :=
  match first_existing
          (model_is_file filesystem)
          (resolution_candidates source_directory import) with
  | None => None
  | Some source_path =>
      match model_read_file filesystem source_path with
      | None => None
      | Some source => model_parse_file filesystem source_path source
      end
  end.

(**
 * @brief Spécialise la résolution par système de fichiers en résolveur d'imports de la grammaire.
 * @param filesystem Modèle des opérations de système de fichiers.
 * @param source_directory Répertoire contenant le fichier source.
 * @return Une fonction qui résout chaque chemin logique d'import en AST optionnel.
 *)
Definition filesystem_resolver
    (filesystem : filesystem_model)
    (source_directory : physical_path) : import_resolver :=
  fun import =>
    resolve_import_from_filesystem filesystem source_directory import.

(**
 * @brief Prouve que tout chemin sélectionné appartient aux candidats et satisfait le test d'existence.
 * @param is_file Prédicat booléen d'existence des fichiers.
 * @param candidates Liste ordonnée des chemins candidats.
 * @param selected Chemin renvoyé par la recherche.
 * @return L'appartenance du chemin aux candidats et la vérité de son test d'existence.
 *)
Lemma first_existing_is_a_true_candidate :
  forall is_file candidates selected,
    first_existing is_file candidates = Some selected ->
    In selected candidates /\ is_file selected = true.
Proof.
  intros is_file candidates.
  induction candidates as [|candidate rest IH]; intros selected Hselected.
  - discriminate.
  - simpl in Hselected.
    destruct (is_file candidate) eqn:Hcandidate.
    + inversion Hselected; subst.
      split.
      * left. reflexivity.
      * exact Hcandidate.
    + destruct (IH selected Hselected) as [Hin Hfile].
      split.
      * right. exact Hin.
      * exact Hfile.
Qed.

(**
 * @brief Prouve qu'un échec de recherche signifie que tous les candidats échouent au test d'existence.
 * @param is_file Prédicat booléen d'existence des fichiers.
 * @param candidates Liste ordonnée des chemins candidats.
 * @return La propriété selon laquelle chaque candidat est déclaré inexistant.
 *)
Lemma first_existing_none_means_no_candidate_exists :
  forall is_file candidates,
    first_existing is_file candidates = None ->
    Forall (fun candidate => is_file candidate = false) candidates.
Proof.
  intros is_file candidates.
  induction candidates as [|candidate rest IH]; intros Hnone.
  - constructor.
  - simpl in Hnone.
    destruct (is_file candidate) eqn:Hcandidate; try discriminate.
    constructor; auto.
Qed.

(**
 * @brief Prouve que la recherche sélectionne immédiatement le premier candidat lorsqu'il existe.
 * @param is_file Prédicat booléen d'existence des fichiers.
 * @param candidate Premier chemin candidat.
 * @param rest Candidats suivants, qui ne sont alors pas examinés.
 * @return La sélection du premier candidat sous l'hypothèse que son test vaut true.
 *)
Lemma first_existing_selects_the_head_when_it_exists :
  forall is_file candidate rest,
    is_file candidate = true ->
    first_existing is_file (candidate :: rest) = Some candidate.
Proof.
  intros is_file candidate rest Hfile.
  simpl. rewrite Hfile. reflexivity.
Qed.

(**
 * @brief Caractérise une résolution réussie par un candidat déclaré, lisible et parsable.
 * @param filesystem Modèle des opérations de système de fichiers.
 * @param source_directory Répertoire contenant le fichier source.
 * @param import Chemin logique de l'import demandé.
 * @param resolved_ast AST obtenu par la résolution.
 * @return L'existence d'un chemin candidat et d'un contenu satisfaisant toutes les étapes de résolution.
 *)
Theorem successful_resolution_uses_a_declared_candidate :
  forall filesystem source_directory import resolved_ast,
    resolve_import_from_filesystem filesystem source_directory import =
      Some resolved_ast ->
    exists source_path source,
      In source_path (resolution_candidates source_directory import) /\
      model_is_file filesystem source_path = true /\
      model_read_file filesystem source_path = Some source /\
      model_parse_file filesystem source_path source = Some resolved_ast.
Proof.
  intros filesystem source_directory import resolved_ast Hresolved.
  unfold resolve_import_from_filesystem in Hresolved.
  destruct (first_existing
              (model_is_file filesystem)
              (resolution_candidates source_directory import))
    as [source_path|] eqn:Hselected; try discriminate.
  destruct (model_read_file filesystem source_path)
    as [source|] eqn:Hread; try discriminate.
  exists source_path, source.
  destruct (first_existing_is_a_true_candidate
              _ _ _ Hselected) as [Hin Hfile].
  repeat split; auto.
Qed.

(**
 * @brief Montre qu'une recherche d'import réussie utilise le premier import correspondant et un candidat valide.
 * @param filesystem Modèle des opérations de système de fichiers.
 * @param source_directory Répertoire contenant le fichier source.
 * @param imports Imports déclarés par le programme.
 * @param parent Nom du parent recherché.
 * @param resolved_ast AST obtenu par la recherche.
 * @return L'existence de l'import choisi, de son fichier candidat et du contenu effectivement parsé.
 *)
Theorem successful_search_import_uses_the_first_matching_import :
  forall filesystem source_directory imports parent resolved_ast,
    search_import
      (filesystem_resolver filesystem source_directory)
      imports parent = Some resolved_ast ->
    exists import source_path source,
      find_import_path imports parent = Some import /\
      In source_path (resolution_candidates source_directory import) /\
      model_is_file filesystem source_path = true /\
      model_read_file filesystem source_path = Some source /\
      model_parse_file filesystem source_path source = Some resolved_ast.
Proof.
  intros filesystem source_directory imports parent resolved_ast Hsearch.
  unfold search_import in Hsearch.
  destruct (find_import_path imports parent) as [import|] eqn:Himport;
    try discriminate.
  unfold filesystem_resolver in Hsearch.
  destruct (successful_resolution_uses_a_declared_candidate
              filesystem source_directory import resolved_ast Hsearch)
    as [source_path [source [Hin [Hfile [Hread Hparse]]]]].
  exists import, source_path, source.
  repeat split; auto.
Qed.

(**
 * @brief Prouve que l'attachement remplace le premier import lorsque son chemin correspond au parent.
 * @param path Chemin de l'import placé en tête.
 * @param imported AST précédemment attaché à cet import.
 * @param rest Imports restants.
 * @param parent Nom du parent recherché.
 * @param parent_file AST à attacher.
 * @return La liste dont le premier import contient désormais l'AST du parent.
 *)
Lemma attach_updates_the_first_matching_import :
  forall path imported rest parent parent_file,
    path_ends_with path parent = true ->
    attach_resolved_parent_to_imports
      (Import path imported :: rest) parent parent_file =
    Import path (Some parent_file) :: rest.
Proof.
  intros path imported rest parent parent_file Hmatches.
  simpl. rewrite Hmatches. reflexivity.
Qed.

(**
 * @brief Prouve que l'attachement conserve un import non correspondant et poursuit la recherche.
 * @param path Chemin de l'import placé en tête.
 * @param imported AST déjà attaché à cet import.
 * @param rest Imports restants.
 * @param parent Nom du parent recherché.
 * @param parent_file AST à attacher lorsqu'une correspondance est trouvée.
 * @return La conservation de l'import de tête suivie de l'attachement récursif dans le reste.
 *)
Lemma attach_skips_a_non_matching_import :
  forall path imported rest parent parent_file,
    path_ends_with path parent = false ->
    attach_resolved_parent_to_imports
      (Import path imported :: rest) parent parent_file =
    Import path imported ::
      attach_resolved_parent_to_imports rest parent parent_file.
Proof.
  intros path imported rest parent parent_file HdoesNotMatch.
  simpl. rewrite HdoesNotMatch. reflexivity.
Qed.

(**
 * @brief Représente l'interface native restante entre un chemin physique et un AST optionnel.
 * @return Le type fonctionnel d'un résolveur natif d'import.
 *)
Definition native_import_resolver : Type := physical_path -> option ast.

(**
 * @brief Exprime l'obligation de pont reliant le résolveur natif au modèle pur du système de fichiers.
 * @param native Résolveur natif fondé sur les opérations réelles de fichiers et le parseur Rust récursif.
 * @param filesystem Modèle pur des opérations natives.
 * @param source_directory Répertoire contenant le fichier source.
 * @return L'égalité point par point des résultats natifs et modélisés pour tout import.
 *)
Definition native_resolver_matches_model
    (native : native_import_resolver)
    (filesystem : filesystem_model)
    (source_directory : physical_path) : Prop :=
  forall import,
    native import =
    resolve_import_from_filesystem filesystem source_directory import.

(**
 * @brief Déduit l'exactitude du résolveur natif une fois l'obligation de pont établie.
 * @param native Résolveur natif à vérifier.
 * @param filesystem Modèle pur correspondant aux opérations natives.
 * @param source_directory Répertoire contenant le fichier source.
 * @param import Chemin logique de l'import demandé.
 * @return L'égalité entre le résultat natif et celui du résolveur issu du modèle.
 *)
Theorem native_resolver_is_exact_after_bridge :
  forall native filesystem source_directory import,
    native_resolver_matches_model native filesystem source_directory ->
    native import =
    filesystem_resolver filesystem source_directory import.
Proof.
  intros native filesystem source_directory import Hbridge.
  apply Hbridge.
Qed.

End ImportResolution.
