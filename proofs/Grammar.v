From Stdlib Require Import Lists.List.
From Stdlib Require Import Strings.String.

Import ListNotations.

(**
 * @brief Regroupe la grammaire SPEADL, son AST et les preuves associées.
 *)
Module MayRustGrammar.

(**
 * @brief Représente les tokens produits par le lexer SPEADL.
 *)
Inductive token : Type :=
| TIdentifier : string -> token
| TDot
| TColon
| TEquals
| TLbrace
| TRbrace
| TLbracket
| TRbracket
| TImport
| TNamespace
| TComponent
| TSpecializes
| TProvides
| TRequires
| TPart
| TBind
| TTo
| TEOF.

(**
 * @brief Référence un service fourni par une part et son nom de service.
 *)
Inductive service_reference : Type :=
| ServiceReference : string -> string -> service_reference.

(**
 * @brief Décrit si un service fourni est local ou délégué à une part.
 *)
Inductive provided_service_implementation : Type :=
| Local
| Delegated : service_reference -> provided_service_implementation.

(**
 * @brief Représente l'AST SPEADL construit par le parseur Rust.
 *)
Inductive ast : Type :=
| Seq : list ast -> ast
| Import : list string -> option ast -> ast
| Namespace : list string -> ast -> ast
| Component :
    string -> option (string * option ast) -> option string -> ast -> ast
| Requires : string -> string -> ast
| Provides : string -> string -> provided_service_implementation -> ast
| Part : string -> string -> option string -> ast -> ast
| Bind : string -> list string -> ast.

(**
 * @brief Représente le nom d'un parent spécialisé et son AST résolu éventuel.
 *)
Definition specialization : Type := (string * option ast)%type.

(**
 * @brief Indique qu'un chemin contient au moins un segment.
 * @param p Chemin à vérifier.
 * @return Une proposition attestant l'existence d'une tête et d'une queue.
 *)
Definition nonempty_path (p : list string) : Prop :=
  exists head tail, p = head :: tail.

(**
 * @brief Vérifie qu'une cible de bind contient un ou deux segments.
 * @param target Cible du bind.
 * @return Une proposition décrivant les deux formes autorisées.
 *)
Definition bind_target_ok (target : list string) : Prop :=
  (exists name, target = [name]) \/
  (exists lhs rhs, target = [lhs; rhs]).

(**
 * @brief Reconnaît un nœud d'import dans l'AST.
 * @param node Nœud à inspecter.
 *)
Definition is_import (node : ast) : Prop :=
  match node with
  | Import _ _ => True
  | _ => False
  end.

(**
 * @brief Reconnaît un nœud de namespace dans l'AST.
 * @param node Nœud à inspecter.
 *)
Definition is_namespace (node : ast) : Prop :=
  match node with
  | Namespace _ _ => True
  | _ => False
  end.

(**
 * @brief Reconnaît un nœud de composant dans l'AST.
 * @param node Nœud à inspecter.
 *)
Definition is_component (node : ast) : Prop :=
  match node with
  | Component _ _ _ _ => True
  | _ => False
  end.

(**
 * @brief Reconnaît les catégories de nœuds autorisées dans un composant.
 * @param node Nœud à inspecter.
 *)
Definition component_item_kind (node : ast) : Prop :=
  match node with
  | Requires _ _ => True
  | Provides _ _ _ => True
  | Part _ _ _ _ => True
  | _ => False
  end.

(**
 * @brief Reconnaît les catégories de nœuds autorisées dans une part.
 * @param node Nœud à inspecter.
 *)
Definition part_item_kind (node : ast) : Prop :=
  match node with
  | Bind _ _ => True
  | _ => False
  end.

(**
 * @brief Recherche récursivement une déclaration provides dans une liste.
 * @param nodes Nœuds du corps d'un composant.
 * @return true si au moins un nœud Provides est présent.
 *)
Fixpoint contains_provides (nodes : list ast) : bool :=
  match nodes with
  | [] => false
  | Provides _ _ _ :: _ => true
  | _ :: rest => contains_provides rest
  end.

(**
 * @brief Définit les invariants structurels d'un AST SPEADL bien formé.
 *)
Inductive ast_wf : ast -> Prop :=
| WfSeq :
    forall nodes,
      Forall ast_wf nodes ->
      ast_wf (Seq nodes)
| WfImport :
    forall path imported,
      nonempty_path path ->
      ast_wf (Import path imported)
| WfNamespace :
    forall path body,
      nonempty_path path ->
      is_component body ->
      ast_wf body ->
      ast_wf (Namespace path body)
| WfComponent :
    forall name specializes generic nodes,
      Forall ast_wf nodes ->
      Forall component_item_kind nodes ->
      contains_provides nodes = true ->
      ast_wf (Component name specializes generic (Seq nodes))
| WfRequires :
    forall name type_name,
      ast_wf (Requires name type_name)
| WfProvides :
    forall name type_name implementation,
      ast_wf (Provides name type_name implementation)
| WfPart :
    forall name type_name generic nodes,
      Forall ast_wf nodes ->
      Forall part_item_kind nodes ->
      ast_wf (Part name type_name generic (Seq nodes))
| WfBind :
    forall name target,
      bind_target_ok target ->
      ast_wf (Bind name target).

(**
 * @brief Définit l'ordre valide des imports et du namespace à la racine.
 *)
Inductive root_items : list ast -> Prop :=
| RootNamespace :
    forall namespace,
      is_namespace namespace ->
      ast_wf namespace ->
      root_items [namespace]
| RootImport :
    forall import rest,
      is_import import ->
      ast_wf import ->
      root_items rest ->
      root_items (import :: rest).

(**
 * @brief Vérifie qu'un programme est une séquence racine bien formée.
 * @param node AST racine à vérifier.
 *)
Definition program_wf (node : ast) : Prop :=
  match node with
  | Seq nodes => Forall ast_wf nodes /\ root_items nodes
  | _ => False
  end.

(**
 * @brief Décrit les AST constructibles conformément à la grammaire SPEADL.
 *)
Inductive grammar_ast : ast -> Prop :=
| GrammarSeq :
    forall nodes,
      Forall grammar_ast nodes ->
      grammar_ast (Seq nodes)
| GrammarImport :
    forall path imported,
      nonempty_path path ->
      grammar_ast (Import path imported)
| GrammarNamespace :
    forall path body,
      nonempty_path path ->
      grammar_ast body ->
      is_component body ->
      grammar_ast (Namespace path body)
| GrammarComponent :
    forall name specializes generic nodes,
      Forall grammar_ast nodes ->
      Forall component_item_kind nodes ->
      contains_provides nodes = true ->
      grammar_ast (Component name specializes generic (Seq nodes))
| GrammarRequires :
    forall name type_name,
      grammar_ast (Requires name type_name)
| GrammarProvides :
    forall name type_name implementation,
      grammar_ast (Provides name type_name implementation)
| GrammarPart :
    forall name type_name generic nodes,
      Forall grammar_ast nodes ->
      Forall part_item_kind nodes ->
      grammar_ast (Part name type_name generic (Seq nodes))
| GrammarBind :
    forall name target,
      bind_target_ok target ->
      grammar_ast (Bind name target).

(**
 * @brief Prouve que tout AST produit par la grammaire respecte ast_wf.
 * @param node AST dérivé par grammar_ast.
 * @return Une preuve de bonne formation structurelle du même AST.
 *)
Lemma grammar_ast_wf :
  forall node, grammar_ast node -> ast_wf node.
Proof.
  fix IH 2.
  intros node Hgrammar.
  destruct Hgrammar as
    [nodes Hnodes
    | path imported Hpath
    | path body Hpath Hbody Hcomponent
    | name specializes generic nodes Hnodes Hkind Hcontains
    | name type_name
    | name type_name implementation
    | name type_name generic nodes Hnodes Hkind
    | name target Htarget].
  - apply WfSeq.
    induction Hnodes as [|node nodes Hnode _ HnodesWf].
    + constructor.
    + constructor.
      * apply IH; exact Hnode.
      * exact HnodesWf.
  - apply WfImport; exact Hpath.
  - apply WfNamespace.
    + exact Hpath.
    + exact Hcomponent.
    + apply IH; exact Hbody.
  - apply WfComponent.
    + assert (HnodesWf : Forall ast_wf nodes).
      { clear Hkind Hcontains.
        induction Hnodes as [|node nodes Hnode _ HnodesWf].
        - constructor.
        - constructor.
          + apply IH; exact Hnode.
          + exact HnodesWf. }
      exact HnodesWf.
    + exact Hkind.
    + exact Hcontains.
  - apply WfRequires.
  - apply WfProvides.
  - apply WfPart.
    + assert (HnodesWf : Forall ast_wf nodes).
      { clear Hkind.
        induction Hnodes as [|node nodes Hnode _ HnodesWf].
        - constructor.
        - constructor.
          + apply IH; exact Hnode.
          + exact HnodesWf. }
      exact HnodesWf.
    + exact Hkind.
  - apply WfBind; exact Htarget.
Qed.

(**
 * @brief Étend grammar_ast_wf à tous les éléments d'une liste.
 * @param nodes Liste d'AST dérivés par la grammaire.
 *)
Lemma grammar_ast_wf_forall :
  forall nodes, Forall grammar_ast nodes -> Forall ast_wf nodes.
Proof.
  intros nodes H.
  induction H; constructor; auto using grammar_ast_wf.
Qed.

(**
 * @brief Combine deux propriétés Forall portant sur des listes concaténées.
 * @param xs Première liste.
 * @param ys Seconde liste.
 *)
Lemma Forall_app_intro :
  forall (A : Type) (P : A -> Prop) (xs ys : list A),
    Forall P xs -> Forall P ys -> Forall P (xs ++ ys).
Proof.
  intros A P xs ys Hxs Hys.
  induction Hxs; simpl.
  - exact Hys.
  - constructor; auto.
Qed.

(**
 * @brief Préserve la présence d'un Provides lors d'une concaténation à droite.
 * @param xs Liste contenant déjà un Provides.
 * @param ys Liste ajoutée à droite.
 *)
Lemma contains_provides_app_left :
  forall xs ys,
    contains_provides xs = true ->
    contains_provides (xs ++ ys) = true.
Proof.
  induction xs as [|x xs IH]; intros ys H; simpl in H; try discriminate.
  destruct x; simpl in *; auto.
Qed.

(**
 * @brief Prouve qu'un chemin réduit à un nom est non vide.
 * @param name Unique segment du chemin.
 *)
Lemma nonempty_path_single :
  forall name, nonempty_path [name].
Proof.
  intros name. exists name, []. reflexivity.
Qed.

(**
 * @brief Prouve que l'ajout d'un segment conserve un chemin non vide.
 * @param path Chemin initial non vide.
 * @param name Segment ajouté en fin de chemin.
 *)
Lemma nonempty_path_app :
  forall path name,
    nonempty_path path -> nonempty_path (path ++ [name]).
Proof.
  intros path name [head [tail ->]].
  exists head, (tail ++ [name]).
  reflexivity.
Qed.

(**
 * @brief Autorise l'arrêt du parsing d'un chemin uniquement hors d'un point.
 * @param input Tokens restant à lire.
 *)
Definition path_tail_can_stop (input : list token) : Prop :=
  match input with
  | TDot :: _ => False
  | _ => True
  end.

(**
 * @brief Autorise l'absence de générique uniquement hors d'un crochet ouvrant.
 * @param input Tokens restant à lire.
 *)
Definition generic_can_stop (input : list token) : Prop :=
  match input with
  | TLbracket :: _ => False
  | _ => True
  end.

(**
 * @brief Autorise l'absence de spécialisation hors du mot-clé specializes.
 * @param input Tokens restant à lire.
 *)
Definition specializes_can_stop (input : list token) : Prop :=
  match input with
  | TSpecializes :: _ => False
  | _ => True
  end.

(**
 * @brief Autorise une implémentation locale uniquement hors du symbole égal.
 * @param input Tokens restant à lire.
 *)
Definition implementation_can_be_local (input : list token) : Prop :=
  match input with
  | TEquals :: _ => False
  | _ => True
  end.

(**
 * @brief Autorise l'arrêt des binds uniquement hors du mot-clé bind.
 * @param input Tokens restant à lire.
 *)
Definition binds_can_stop (input : list token) : Prop :=
  match input with
  | TBind :: _ => False
  | _ => True
  end.

(**
 * @brief Autorise une cible courte uniquement lorsqu'aucun point ne suit.
 * @param input Tokens situés après le premier segment de cible.
 *)
Definition bind_target_can_stop (input : list token) : Prop :=
  match input with
  | TDot :: _ => False
  | _ => True
  end.

(**
 * @brief Autorise l'arrêt des parts uniquement hors du mot-clé part.
 * @param input Tokens restant à lire.
 *)
Definition parts_can_stop (input : list token) : Prop :=
  match input with
  | TPart :: _ => False
  | _ => True
  end.

(**
 * @brief Autorise l'arrêt des provides uniquement hors du mot-clé provides.
 * @param input Tokens restant à lire.
 *)
Definition provides_can_stop (input : list token) : Prop :=
  match input with
  | TProvides :: _ => False
  | _ => True
  end.

(**
 * @brief Autorise l'arrêt des requires uniquement hors du mot-clé requires.
 * @param input Tokens restant à lire.
 *)
Definition requires_can_stop (input : list token) : Prop :=
  match input with
  | TRequires :: _ => False
  | _ => True
  end.

(**
 * @brief Autorise l'arrêt des imports uniquement hors du mot-clé import.
 * @param input Tokens restant à lire.
 *)
Definition imports_can_stop (input : list token) : Prop :=
  match input with
  | TImport :: _ => False
  | _ => True
  end.

(**
 * @brief Parse récursivement les segments pointés suivant un chemin initial.
 *)
Inductive parses_path_tail :
  list string -> list token -> list string -> list token -> Prop :=
| ParsesPathStop :
    forall acc input,
      path_tail_can_stop input ->
      parses_path_tail acc input acc input
| ParsesPathDot :
    forall acc name input path rest,
      parses_path_tail (acc ++ [name]) input path rest ->
      parses_path_tail acc (TDot :: TIdentifier name :: input) path rest.

(**
 * @brief Parse un chemin non vide commençant par un identifiant.
 *)
Inductive parses_path : list token -> list string -> list token -> Prop :=
| ParsesPath :
    forall name input path rest,
      parses_path_tail [name] input path rest ->
      parses_path (TIdentifier name :: input) path rest.

(**
 * @brief Parse un paramètre générique optionnel entre crochets.
 *)
Inductive parses_generic :
  list token -> option string -> list token -> Prop :=
| ParsesGenericNone :
    forall input,
      generic_can_stop input ->
      parses_generic input None input
| ParsesGenericSome :
    forall name rest,
      parses_generic (TLbracket :: TIdentifier name :: TRbracket :: rest)
                     (Some name) rest.

(**
 * @brief Parse une spécialisation optionnelle et résout son AST parent.
 * @param resolve Fonction de résolution du nom du parent.
 *)
Inductive parses_specializes (resolve : string -> option ast) :
  list token -> option specialization -> list token -> Prop :=
| ParsesSpecializesNone :
    forall input,
      specializes_can_stop input ->
      parses_specializes resolve input None input
| ParsesSpecializesSome :
    forall name rest,
      parses_specializes resolve
        (TSpecializes :: TIdentifier name :: rest)
        (Some (name, resolve name)) rest.

(**
 * @brief Parse l'implémentation locale ou déléguée d'un service fourni.
 *)
Inductive parses_implementation :
  list token -> provided_service_implementation -> list token -> Prop :=
| ParsesImplementationLocal :
    forall input,
      implementation_can_be_local input ->
      parses_implementation input Local input
| ParsesImplementationDelegated :
    forall part_name service_name rest,
      parses_implementation
        (TEquals :: TIdentifier part_name :: TDot ::
         TIdentifier service_name :: rest)
        (Delegated (ServiceReference part_name service_name)) rest.

(**
 * @brief Parse zéro ou plusieurs binds dans le corps d'une part.
 *)
Inductive parses_binds : list token -> list ast -> list token -> Prop :=
| ParsesBindsStop :
    forall input,
      binds_can_stop input ->
      parses_binds input [] input
| ParsesBindOne :
    forall name target input binds rest,
      bind_target_can_stop input ->
      parses_binds input binds rest ->
      parses_binds (TBind :: TIdentifier name :: TTo ::
                    TIdentifier target :: input)
                   (Bind name [target] :: binds) rest
| ParsesBindTwo :
    forall name target field input binds rest,
      parses_binds input binds rest ->
      parses_binds (TBind :: TIdentifier name :: TTo ::
                    TIdentifier target :: TDot :: TIdentifier field :: input)
                   (Bind name [target; field] :: binds) rest.

(**
 * @brief Parse zéro ou plusieurs déclarations de parts et leurs binds.
 *)
Inductive parses_parts : list token -> list ast -> list token -> Prop :=
| ParsesPartsStop :
    forall input,
      parts_can_stop input ->
      parses_parts input [] input
| ParsesPartCons :
    forall name type_name generic after_type binds body after_part parts rest,
      parses_generic after_type generic (TLbrace :: body) ->
      parses_binds body binds (TRbrace :: after_part) ->
      parses_parts after_part parts rest ->
      parses_parts (TPart :: TIdentifier name :: TColon ::
                    TIdentifier type_name :: after_type)
                   (Part name type_name generic (Seq binds) :: parts) rest.

(**
 * @brief Parse une séquence non vide de déclarations provides.
 *)
Inductive parses_provide_entries :
  list token -> list ast -> list token -> Prop :=
| ParsesProvideLast :
    forall name type_name after_type implementation rest,
      parses_implementation after_type implementation rest ->
      provides_can_stop rest ->
      parses_provide_entries
        (TProvides :: TIdentifier name :: TColon ::
         TIdentifier type_name :: after_type)
        [Provides name type_name implementation] rest
| ParsesProvideMore :
    forall name type_name after_type implementation after_implementation
           entries rest,
      parses_implementation after_type implementation after_implementation ->
      parses_provide_entries after_implementation entries rest ->
      parses_provide_entries
        (TProvides :: TIdentifier name :: TColon ::
         TIdentifier type_name :: after_type)
        (Provides name type_name implementation :: entries) rest.

(**
 * @brief Parse les provides obligatoires puis les parts éventuelles.
 *)
Inductive parses_provides : list token -> list ast -> list token -> Prop :=
| ParsesProvides :
    forall input provides after_provides parts rest,
      parses_provide_entries input provides after_provides ->
      parses_parts after_provides parts rest ->
      parses_provides input (provides ++ parts) rest.

(**
 * @brief Parse les requires éventuels suivis des provides obligatoires.
 *)
Inductive parses_requires : list token -> list ast -> list token -> Prop :=
| ParsesRequiresStop :
    forall input nodes rest,
      requires_can_stop input ->
      parses_provides input nodes rest ->
      parses_requires input nodes rest
| ParsesRequiresCons :
    forall name type_name input nodes rest,
      parses_requires input nodes rest ->
      parses_requires (TRequires :: TIdentifier name :: TColon ::
                       TIdentifier type_name :: input)
                      (Requires name type_name :: nodes) rest.

(**
 * @brief Parse un composant complet et construit son AST.
 * @param resolve Fonction résolvant l'AST du composant parent spécialisé.
 *)
Inductive parses_component (resolve : string -> option ast) :
  list token -> ast -> list token -> Prop :=
| ParsesComponent :
    forall name after_name specializes after_specializes generic nodes body rest,
      parses_specializes resolve after_name specializes after_specializes ->
      parses_generic after_specializes generic (TLbrace :: body) ->
      parses_requires body nodes (TRbrace :: rest) ->
      parses_component resolve
        (TComponent :: TIdentifier name :: after_name)
        (Component name specializes generic (Seq nodes))
        rest.

(**
 * @brief Parse les imports précédant un namespace.
 *)
Inductive parses_imports : list token -> list ast -> list token -> Prop :=
| ParsesImportsStop :
    forall input,
      imports_can_stop input ->
      parses_imports input [] input
| ParsesImportCons :
    forall input path after_path imports rest,
      parses_path input path after_path ->
      parses_imports after_path imports rest ->
      parses_imports (TImport :: input) (Import path None :: imports) rest.

(**
 * @brief Extrait les chemins de tous les nœuds Import d'une liste d'AST.
 * @param imports Nœuds d'import parsés.
 *)
Fixpoint import_paths (imports : list ast) : list (list string) :=
  match imports with
  | [] => []
  | Import path _ :: rest => path :: import_paths rest
  | _ :: rest => import_paths rest
  end.

(**
 * @brief Teste si le dernier segment d'un chemin correspond à un nom.
 * @param path Chemin d'import.
 * @param name Nom recherché.
 *)
Definition path_ends_with (path : list string) (name : string) : bool :=
  match rev path with
  | [] => false
  | last_name :: _ => String.eqb last_name name
  end.

(**
 * @brief Recherche le premier chemin d'import se terminant par un nom.
 * @param paths Chemins d'import disponibles.
 * @param name Nom du composant recherché.
 *)
Fixpoint find_import_path
    (paths : list (list string)) (name : string) : option (list string) :=
  match paths with
  | [] => None
  | path :: rest =>
      if path_ends_with path name
      then Some path
      else find_import_path rest name
  end.

(**
 * @brief Type d'une fonction résolvant un chemin d'import vers un AST.
 *)
Definition import_resolver : Type := list string -> option ast.

(**
 * @brief Résout le premier import dont le dernier segment correspond au nom.
 * @param resolve Résolveur de chemins abstrait.
 * @param paths Chemins d'import disponibles.
 * @param name Nom du parent recherché.
 *)
Definition search_import
    (resolve : import_resolver)
    (paths : list (list string))
    (name : string) : option ast :=
  match find_import_path paths name with
  | Some path => resolve path
  | None => None
  end.

(**
 * @brief Attache l'AST parent au premier import correspondant.
 * @param imports Nœuds d'import à parcourir.
 * @param parent Nom du composant parent.
 * @param parent_file AST résolu du parent.
 *)
Fixpoint attach_resolved_parent_to_imports
    (imports : list ast) (parent : string) (parent_file : ast) : list ast :=
  match imports with
  | [] => []
  | Import path imported :: rest =>
      if path_ends_with path parent
      then Import path (Some parent_file) :: rest
      else Import path imported ::
           attach_resolved_parent_to_imports rest parent parent_file
  | node :: rest =>
      node :: attach_resolved_parent_to_imports rest parent parent_file
  end.

(**
 * @brief Attache aux imports le parent résolu d'un composant spécialisé.
 * @param imports Nœuds d'import à enrichir.
 * @param component Composant éventuellement spécialisé.
 *)
Definition attach_specialized_parent_to_imports
    (imports : list ast) (component : ast) : list ast :=
  match component with
  | Component _ (Some (parent, Some parent_file)) _ _ =>
      attach_resolved_parent_to_imports imports parent parent_file
  | _ => imports
  end.

(**
 * @brief Parse un programme SPEADL complet composé d'imports et d'un namespace.
 * @param resolve Résolveur abstrait utilisé pour les imports spécialisés.
 *)
Inductive parses_namespace (resolve : import_resolver) :
  list token -> ast -> list token -> Prop :=
| ParsesNamespace :
    forall input imports after_imports path after_path component rest,
      parses_imports input imports (TNamespace :: after_imports) ->
      parses_path after_imports path (TLbrace :: after_path) ->
      parses_component
        (fun name => search_import resolve (import_paths imports) name)
        after_path component (TRbrace :: rest) ->
      parses_namespace resolve input
        (Seq (attach_specialized_parent_to_imports imports component ++
              [Namespace path component]))
        rest.

(**
 * @brief Résolveur de test ne retournant aucun AST importé.
 *)
Definition no_import_resolution : import_resolver := fun _ => None.

Open Scope string_scope.

(**
 * @brief Vérifie constructivement le parsing d'un namespace minimal valide.
 *)
Example parses_minimal_namespace :
  parses_namespace no_import_resolution
    [TNamespace; TIdentifier "demo"; TLbrace;
     TComponent; TIdentifier "Main"; TLbrace;
     TProvides; TIdentifier "service"; TColon; TIdentifier "Service";
     TRbrace; TRbrace; TEOF]
    (Seq
      [Namespace ["demo"]
        (Component "Main" None None
          (Seq [Provides "service" "Service" Local]))])
    [TEOF].
Proof.
  eapply ParsesNamespace with
    (imports := [])
    (path := ["demo"])
    (component :=
      Component "Main" None None
        (Seq [Provides "service" "Service" Local])).
  - apply ParsesImportsStop. simpl. exact I.
  - apply ParsesPath. apply ParsesPathStop. simpl. exact I.
  - eapply ParsesComponent.
    + apply ParsesSpecializesNone. simpl. exact I.
    + apply ParsesGenericNone. simpl. exact I.
    + apply ParsesRequiresStop.
      * simpl. exact I.
      * eapply ParsesProvides with
          (provides := [Provides "service" "Service" Local])
          (parts := []).
        -- apply ParsesProvideLast.
           ++ apply ParsesImplementationLocal. simpl. exact I.
           ++ simpl. exact I.
        -- apply ParsesPartsStop. simpl. exact I.
Qed.

(**
 * @brief Vérifie le parsing d'une part générique avec deux formes de bind.
 *)
Example parses_part_with_binds :
  parses_parts
    [TPart; TIdentifier "worker"; TColon; TIdentifier "Worker";
     TLbracket; TIdentifier "Config"; TRbracket; TLbrace;
     TBind; TIdentifier "local"; TTo; TIdentifier "service";
     TBind; TIdentifier "delegated"; TTo; TIdentifier "child"; TDot;
       TIdentifier "service";
     TRbrace; TEOF]
    [Part "worker" "Worker" (Some "Config")
      (Seq
        [Bind "local" ["service"];
         Bind "delegated" ["child"; "service"]])]
    [TEOF].
Proof.
  eapply ParsesPartCons.
  - apply ParsesGenericSome.
  - apply ParsesBindOne.
    + simpl. exact I.
    + apply ParsesBindTwo.
      apply ParsesBindsStop. simpl. exact I.
  - apply ParsesPartsStop. simpl. exact I.
Qed.

(**
 * @brief Vérifie qu'un parent résolu est attaché à l'import correspondant.
 *)
Example attaches_resolved_parent_to_matching_import :
  attach_specialized_parent_to_imports
    [Import ["example"; "Parent"] None]
    (Component "Child" (Some ("Parent", Some (Seq []))) None
      (Seq [Provides "service" "Service" Local])) =
  [Import ["example"; "Parent"] (Some (Seq []))].
Proof.
  reflexivity.
Qed.

Close Scope string_scope.

(**
 * @brief Prouve qu'un chemin accumulé reste non vide après parsing de sa queue.
 * @param acc Accumulateur de segments déjà non vide.
 * @param input Tokens avant parsing.
 * @param path Chemin final produit.
 * @param rest Tokens restant après parsing.
 *)
Lemma parses_path_tail_nonempty :
  forall acc input path rest,
    nonempty_path acc ->
    parses_path_tail acc input path rest ->
    nonempty_path path.
Proof.
  intros acc input path rest Hacc Hparse.
  induction Hparse; auto using nonempty_path_app.
Qed.

(**
 * @brief Prouve que tout chemin parsé contient au moins un segment.
 * @param input Tokens avant parsing.
 * @param path Chemin produit.
 * @param rest Tokens restant après parsing.
 *)
Lemma parses_path_nonempty :
  forall input path rest,
    parses_path input path rest ->
    nonempty_path path.
Proof.
  intros input path rest Hparse.
  inversion Hparse; subst.
  eauto using parses_path_tail_nonempty, nonempty_path_single.
Qed.

(**
 * @brief Prouve que tous les binds parsés sont grammaticaux et bien typés.
 * @param input Tokens avant parsing.
 * @param binds AST de binds produit.
 * @param rest Tokens restant après parsing.
 *)
Lemma parses_binds_sound :
  forall input binds rest,
    parses_binds input binds rest ->
    Forall grammar_ast binds /\ Forall part_item_kind binds.
Proof.
  intros input binds rest Hparse.
  induction Hparse.
  - split; constructor.
  - destruct IHHparse as [Hgrammar Hkind].
    split.
    + constructor.
      * apply GrammarBind. left. exists target. reflexivity.
      * exact Hgrammar.
    + constructor; simpl; auto.
  - destruct IHHparse as [Hgrammar Hkind].
    split.
    + constructor.
      * apply GrammarBind. right. exists target, field. reflexivity.
      * exact Hgrammar.
    + constructor; simpl; auto.
Qed.

(**
 * @brief Prouve que toutes les parts parsées respectent la grammaire du composant.
 * @param input Tokens avant parsing.
 * @param parts AST de parts produit.
 * @param rest Tokens restant après parsing.
 *)
Lemma parses_parts_sound :
  forall input parts rest,
    parses_parts input parts rest ->
    Forall grammar_ast parts /\ Forall component_item_kind parts.
Proof.
  intros input parts rest Hparse.
  induction Hparse.
  - split; constructor.
  - destruct (parses_binds_sound body binds (TRbrace :: after_part) H0)
      as [HbindGrammar HbindKind].
    destruct IHHparse as [HpartGrammar HpartKind].
    split; constructor; simpl; auto using GrammarPart.
Qed.

(**
 * @brief Prouve la correction d'une séquence non vide de provides parsés.
 * @param input Tokens avant parsing.
 * @param entries AST de provides produit.
 * @param rest Tokens restant après parsing.
 *)
Lemma parses_provide_entries_sound :
  forall input entries rest,
    parses_provide_entries input entries rest ->
    Forall grammar_ast entries /\
    Forall component_item_kind entries /\
    contains_provides entries = true.
Proof.
  intros input entries rest Hparse.
  induction Hparse.
  - split.
    + constructor.
      * apply GrammarProvides.
      * constructor.
    + split.
      * constructor.
        -- simpl. exact I.
        -- constructor.
      * reflexivity.
  - destruct IHHparse as [Hgrammar [Hkind Hcontains]].
    split.
    + constructor.
      * apply GrammarProvides.
      * exact Hgrammar.
    + split.
      * constructor.
        -- simpl. exact I.
        -- exact Hkind.
      * reflexivity.
Qed.

(**
 * @brief Prouve la correction des provides suivis des parts éventuelles.
 * @param input Tokens avant parsing.
 * @param nodes AST de provides et parts produit.
 * @param rest Tokens restant après parsing.
 *)
Lemma parses_provides_sound :
  forall input nodes rest,
    parses_provides input nodes rest ->
    Forall grammar_ast nodes /\
    Forall component_item_kind nodes /\
    contains_provides nodes = true.
Proof.
  intros input nodes rest Hparse.
  destruct Hparse as [input provides after_provides parts rest Hentries Hparts].
  destruct (parses_provide_entries_sound input provides after_provides Hentries)
    as [HprovideGrammar [HprovideKind Hcontains]].
  destruct (parses_parts_sound after_provides parts rest Hparts)
    as [HpartGrammar HpartKind].
  repeat split.
  - apply Forall_app_intro; auto.
  - apply Forall_app_intro; auto.
  - apply contains_provides_app_left; auto.
Qed.

(**
 * @brief Prouve la correction des requires suivis du corps obligatoire.
 * @param input Tokens avant parsing.
 * @param nodes AST du corps de composant produit.
 * @param rest Tokens restant après parsing.
 *)
Lemma parses_requires_sound :
  forall input nodes rest,
    parses_requires input nodes rest ->
    Forall grammar_ast nodes /\
    Forall component_item_kind nodes /\
    contains_provides nodes = true.
Proof.
  intros input nodes rest Hparse.
  induction Hparse.
  - eauto using parses_provides_sound.
  - destruct IHHparse as [Hgrammar [Hkind Hcontains]].
    split.
    + constructor; auto using GrammarRequires.
    + split.
      * constructor; simpl; auto.
      * exact Hcontains.
Qed.

(**
 * @brief Prouve qu'un composant parsé produit un AST grammatical et bien formé.
 * @param resolve Résolveur de spécialisation.
 * @param input Tokens avant parsing.
 * @param component AST de composant produit.
 * @param rest Tokens restant après parsing.
 *)
Theorem parses_component_sound :
  forall resolve input component rest,
    parses_component resolve input component rest ->
    grammar_ast component /\ ast_wf component /\ is_component component.
Proof.
  intros resolve input component rest Hparse.
  destruct Hparse as
    [name after_name specializes after_specializes generic nodes body rest
     Hspecializes Hgeneric Hrequires].
  destruct (parses_requires_sound body nodes (TRbrace :: rest) Hrequires)
    as [Hgrammar [Hkind Hcontains]].
  assert (HcomponentGrammar :
    grammar_ast (Component name specializes generic (Seq nodes))).
  { apply GrammarComponent; auto. }
  split.
  - exact HcomponentGrammar.
  - split.
    + apply grammar_ast_wf; exact HcomponentGrammar.
    + simpl. exact I.
Qed.

(**
 * @brief Prouve que les imports parsés sont grammaticaux et bien formés.
 * @param input Tokens avant parsing.
 * @param imports AST d'imports produit.
 * @param rest Tokens restant après parsing.
 *)
Lemma parses_imports_sound :
  forall input imports rest,
    parses_imports input imports rest ->
    Forall grammar_ast imports /\
    Forall ast_wf imports /\
    Forall is_import imports.
Proof.
  intros input imports rest Hparse.
  induction Hparse.
  - repeat split; constructor.
  - assert (Hpath : nonempty_path path)
      by eauto using parses_path_nonempty.
    destruct IHHparse as [Hgrammar [Hwf Hkind]].
    repeat split; constructor; simpl; auto using GrammarImport, grammar_ast_wf.
Qed.

(**
 * @brief Prouve que l'attachement d'un parent préserve les invariants des imports.
 * @param imports Liste d'imports initiale.
 * @param parent Nom du composant parent.
 * @param parent_file AST résolu du parent.
 *)
Lemma attach_resolved_parent_to_imports_preserves :
  forall imports parent parent_file,
    Forall grammar_ast imports ->
    Forall ast_wf imports ->
    Forall is_import imports ->
    Forall grammar_ast
      (attach_resolved_parent_to_imports imports parent parent_file) /\
    Forall ast_wf
      (attach_resolved_parent_to_imports imports parent parent_file) /\
    Forall is_import
      (attach_resolved_parent_to_imports imports parent parent_file).
Proof.
  induction imports as [|node imports IH];
    intros parent parent_file Hgrammar Hwf Hkind.
  - repeat split; constructor.
  - inversion Hgrammar as [|? ? HnodeGrammar HrestGrammar]; subst.
    inversion Hwf as [|? ? HnodeWf HrestWf]; subst.
    inversion Hkind as [|? ? HnodeKind HrestKind]; subst.
    destruct node as
      [nodes
      | path imported
      | namespace_path namespace_body
      | name specializes generic component_body
      | required_name required_type
      | provided_name provided_type implementation
      | part_name part_type part_generic part_body
      | bind_name bind_target];
      simpl in HnodeKind; try contradiction.
    simpl.
    destruct (path_ends_with path parent) eqn:Hends.
    + assert (Hpath : nonempty_path path).
      { inversion HnodeGrammar; assumption. }
      repeat split.
      * constructor.
        -- apply GrammarImport. exact Hpath.
        -- exact HrestGrammar.
      * constructor.
        -- apply WfImport. exact Hpath.
        -- exact HrestWf.
      * constructor.
        -- simpl. exact I.
        -- exact HrestKind.
    + destruct (IH parent parent_file HrestGrammar HrestWf HrestKind)
        as [HattachedGrammar [HattachedWf HattachedKind]].
      repeat split; constructor; simpl; auto.
Qed.

(**
 * @brief Prouve que l'enrichissement selon un composant préserve les imports.
 * @param imports Liste d'imports initiale.
 * @param component Composant éventuellement spécialisé.
 *)
Lemma attach_specialized_parent_to_imports_preserves :
  forall imports component,
    Forall grammar_ast imports ->
    Forall ast_wf imports ->
    Forall is_import imports ->
    Forall grammar_ast
      (attach_specialized_parent_to_imports imports component) /\
    Forall ast_wf
      (attach_specialized_parent_to_imports imports component) /\
    Forall is_import
      (attach_specialized_parent_to_imports imports component).
Proof.
  intros imports component Hgrammar Hwf Hkind.
  destruct component as
    [nodes
    | import_path imported
    | namespace_path namespace_body
    | name specializes generic component_body
    | required_name required_type
    | provided_name provided_type implementation
    | part_name part_type part_generic part_body
    | bind_name bind_target];
    simpl; try (repeat split; assumption).
  destruct specializes as [[parent parent_file]|]; simpl.
  - destruct parent_file; simpl.
    + apply attach_resolved_parent_to_imports_preserves; assumption.
    + repeat split; assumption.
  - repeat split; assumption.
Qed.

(**
 * @brief Construit l'invariant racine à partir d'imports et d'un namespace.
 * @param imports Imports bien formés placés en tête.
 * @param namespace Namespace bien formé placé en dernier.
 *)
Lemma root_items_imports_namespace :
  forall imports namespace,
    Forall ast_wf imports ->
    Forall is_import imports ->
    ast_wf namespace ->
    is_namespace namespace ->
    root_items (imports ++ [namespace]).
Proof.
  induction imports as [|import imports IH]; intros namespace Hwf Hkind Hnswf Hnskind.
  - constructor; auto.
  - inversion Hwf; subst.
    inversion Hkind; subst.
    simpl. constructor; auto.
Qed.

(**
 * @brief Prouve la correction globale du parsing d'un programme SPEADL.
 * @param resolve Résolveur abstrait des imports.
 * @param input Tokens avant parsing.
 * @param program AST racine produit.
 * @param rest Tokens restant après parsing.
 * @return Les preuves grammar_ast, ast_wf et program_wf du programme.
 *)
Theorem parses_namespace_sound :
  forall resolve input program rest,
    parses_namespace resolve input program rest ->
    grammar_ast program /\ ast_wf program /\ program_wf program.
Proof.
  intros resolve input program rest Hparse.
  destruct Hparse as
    [input imports after_imports path after_path component rest
     Himports HpathParse HcomponentParse].
  destruct (parses_imports_sound input imports (TNamespace :: after_imports) Himports)
    as [HimportsGrammar [HimportsWf HimportsKind]].
  assert (Hpath : nonempty_path path)
    by eauto using parses_path_nonempty.
  destruct (parses_component_sound
              (fun name => search_import resolve (import_paths imports) name)
              after_path component (TRbrace :: rest) HcomponentParse)
    as [HcomponentGrammar [HcomponentWf HcomponentKind]].
  destruct (attach_specialized_parent_to_imports_preserves
              imports component HimportsGrammar HimportsWf HimportsKind)
    as [HattachedGrammar [HattachedWf HattachedKind]].
  assert (HnamespaceGrammar : grammar_ast (Namespace path component)).
  { apply GrammarNamespace; auto. }
  assert (HnamespaceWf : ast_wf (Namespace path component)).
  { apply grammar_ast_wf; exact HnamespaceGrammar. }
  split.
  - apply GrammarSeq. apply Forall_app_intro.
    + exact HattachedGrammar.
    + constructor.
      * exact HnamespaceGrammar.
      * constructor.
  - split.
    + apply WfSeq. apply Forall_app_intro.
      * exact HattachedWf.
      * constructor.
        -- exact HnamespaceWf.
        -- constructor.
    + simpl. split.
      * apply Forall_app_intro.
        -- exact HattachedWf.
        -- constructor.
           ++ exact HnamespaceWf.
           ++ constructor.
      * apply root_items_imports_namespace; auto.
        simpl. exact I.
Qed.

End MayRustGrammar.
