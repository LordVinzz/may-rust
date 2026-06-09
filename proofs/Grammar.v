From Stdlib Require Import Lists.List.
From Stdlib Require Import Strings.String.

Import ListNotations.

Module MayRustGrammar.

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

Inductive ast : Type :=
| Seq : list ast -> ast
| Import : list string -> ast
| Namespace : list string -> ast -> ast
| Component : string -> option string -> option string -> ast -> ast
| Requires : string -> string -> ast
| Provides : string -> string -> option (list string) -> ast
| Part : string -> string -> option string -> ast -> ast
| Bind : string -> list string -> ast.

Definition nonempty_path (p : list string) : Prop :=
  exists head tail, p = head :: tail.

Definition source_ok (src : option (list string)) : Prop :=
  src = None \/ exists lhs rhs, src = Some [lhs; rhs].

Definition bind_target_ok (target : list string) : Prop :=
  (exists name, target = [name]) \/
  (exists lhs rhs, target = [lhs; rhs]).

Definition is_import (node : ast) : Prop :=
  match node with
  | Import _ => True
  | _ => False
  end.

Definition is_namespace (node : ast) : Prop :=
  match node with
  | Namespace _ _ => True
  | _ => False
  end.

Definition is_component (node : ast) : Prop :=
  match node with
  | Component _ _ _ _ => True
  | _ => False
  end.

Definition component_item_kind (node : ast) : Prop :=
  match node with
  | Requires _ _ => True
  | Provides _ _ _ => True
  | Part _ _ _ _ => True
  | _ => False
  end.

Definition part_item_kind (node : ast) : Prop :=
  match node with
  | Bind _ _ => True
  | _ => False
  end.

Fixpoint contains_provides (nodes : list ast) : bool :=
  match nodes with
  | [] => false
  | Provides _ _ _ :: _ => true
  | _ :: rest => contains_provides rest
  end.

Inductive ast_wf : ast -> Prop :=
| WfSeq :
    forall nodes,
      Forall ast_wf nodes ->
      ast_wf (Seq nodes)
| WfImport :
    forall path,
      nonempty_path path ->
      ast_wf (Import path)
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
    forall name type_name source,
      source_ok source ->
      ast_wf (Provides name type_name source)
| WfPart :
    forall name type_name generic nodes,
      Forall ast_wf nodes ->
      Forall part_item_kind nodes ->
      ast_wf (Part name type_name generic (Seq nodes))
| WfBind :
    forall name target,
      bind_target_ok target ->
      ast_wf (Bind name target).

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

Definition program_wf (node : ast) : Prop :=
  match node with
  | Seq nodes => Forall ast_wf nodes /\ root_items nodes
  | _ => False
  end.

Inductive grammar_ast : ast -> Prop :=
| GrammarSeq :
    forall nodes,
      Forall grammar_ast nodes ->
      grammar_ast (Seq nodes)
| GrammarImport :
    forall path,
      nonempty_path path ->
      grammar_ast (Import path)
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
    forall name type_name source,
      source_ok source ->
      grammar_ast (Provides name type_name source)
| GrammarPart :
    forall name type_name generic nodes,
      Forall grammar_ast nodes ->
      Forall part_item_kind nodes ->
      grammar_ast (Part name type_name generic (Seq nodes))
| GrammarBind :
    forall name target,
      bind_target_ok target ->
      grammar_ast (Bind name target).

Lemma grammar_ast_wf :
  forall node, grammar_ast node -> ast_wf node.
Proof.
  fix IH 2.
  intros node Hgrammar.
  destruct Hgrammar as
    [nodes Hnodes
    | path Hpath
    | path body Hpath Hbody Hcomponent
    | name specializes generic nodes Hnodes Hkind Hcontains
    | name type_name
    | name type_name source Hsource
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
  - apply WfProvides; exact Hsource.
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

Lemma grammar_ast_wf_forall :
  forall nodes, Forall grammar_ast nodes -> Forall ast_wf nodes.
Proof.
  intros nodes H.
  induction H; constructor; auto using grammar_ast_wf.
Qed.

Lemma Forall_app_intro :
  forall (A : Type) (P : A -> Prop) (xs ys : list A),
    Forall P xs -> Forall P ys -> Forall P (xs ++ ys).
Proof.
  intros A P xs ys Hxs Hys.
  induction Hxs; simpl.
  - exact Hys.
  - constructor; auto.
Qed.

Lemma contains_provides_app_left :
  forall xs ys,
    contains_provides xs = true ->
    contains_provides (xs ++ ys) = true.
Proof.
  induction xs as [|x xs IH]; intros ys H; simpl in H; try discriminate.
  destruct x; simpl in *; auto.
Qed.

Lemma nonempty_path_single :
  forall name, nonempty_path [name].
Proof.
  intros name. exists name, []. reflexivity.
Qed.

Lemma nonempty_path_app :
  forall path name,
    nonempty_path path -> nonempty_path (path ++ [name]).
Proof.
  intros path name [head [tail ->]].
  exists head, (tail ++ [name]).
  reflexivity.
Qed.

Inductive parses_path_tail :
  list string -> list token -> list string -> list token -> Prop :=
| ParsesPathStop :
    forall acc input,
      parses_path_tail acc input acc input
| ParsesPathDot :
    forall acc name input path rest,
      parses_path_tail (acc ++ [name]) input path rest ->
      parses_path_tail acc (TDot :: TIdentifier name :: input) path rest.

Inductive parses_path : list token -> list string -> list token -> Prop :=
| ParsesPath :
    forall name input path rest,
      parses_path_tail [name] input path rest ->
      parses_path (TIdentifier name :: input) path rest.

Inductive parses_generic :
  list token -> option string -> list token -> Prop :=
| ParsesGenericNone :
    forall input,
      parses_generic input None input
| ParsesGenericSome :
    forall name rest,
      parses_generic (TLbracket :: TIdentifier name :: TRbracket :: rest)
                     (Some name) rest.

Inductive parses_specializes :
  list token -> option string -> list token -> Prop :=
| ParsesSpecializesNone :
    forall input,
      parses_specializes input None input
| ParsesSpecializesSome :
    forall name rest,
      parses_specializes (TSpecializes :: TIdentifier name :: rest)
                         (Some name) rest.

Inductive parses_source :
  list token -> option (list string) -> list token -> Prop :=
| ParsesSourceNone :
    forall input,
      parses_source input None input
| ParsesSourcePath :
    forall lhs rhs rest,
      parses_source (TEquals :: TIdentifier lhs :: TDot ::
                     TIdentifier rhs :: rest)
                    (Some [lhs; rhs]) rest.

Inductive parses_binds : list token -> list ast -> list token -> Prop :=
| ParsesBindsStop :
    forall input,
      parses_binds input [] input
| ParsesBindOne :
    forall name target input binds rest,
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

Inductive parses_parts : list token -> list ast -> list token -> Prop :=
| ParsesPartsStop :
    forall input,
      parses_parts input [] input
| ParsesPartCons :
    forall name type_name generic after_type binds after_body parts rest,
      parses_generic after_type generic (TLbrace :: after_body) ->
      parses_binds after_body binds (TRbrace :: after_type) ->
      parses_parts after_type parts rest ->
      parses_parts (TPart :: TIdentifier name :: TColon ::
                    TIdentifier type_name :: after_type)
                   (Part name type_name generic (Seq binds) :: parts) rest.

Inductive parses_provide_entries :
  list token -> list ast -> list token -> Prop :=
| ParsesProvideLast :
    forall name type_name after_type source rest,
      parses_source after_type source rest ->
      parses_provide_entries
        (TProvides :: TIdentifier name :: TColon ::
         TIdentifier type_name :: after_type)
        [Provides name type_name source] rest
| ParsesProvideMore :
    forall name type_name after_type source after_source entries rest,
      parses_source after_type source after_source ->
      parses_provide_entries after_source entries rest ->
      parses_provide_entries
        (TProvides :: TIdentifier name :: TColon ::
         TIdentifier type_name :: after_type)
        (Provides name type_name source :: entries) rest.

Inductive parses_provides : list token -> list ast -> list token -> Prop :=
| ParsesProvides :
    forall input provides after_provides parts rest,
      parses_provide_entries input provides after_provides ->
      parses_parts after_provides parts rest ->
      parses_provides input (provides ++ parts) rest.

Inductive parses_requires : list token -> list ast -> list token -> Prop :=
| ParsesRequiresStop :
    forall input nodes rest,
      parses_provides input nodes rest ->
      parses_requires input nodes rest
| ParsesRequiresCons :
    forall name type_name input nodes rest,
      parses_requires input nodes rest ->
      parses_requires (TRequires :: TIdentifier name :: TColon ::
                       TIdentifier type_name :: input)
                      (Requires name type_name :: nodes) rest.

Inductive parses_component : list token -> ast -> list token -> Prop :=
| ParsesComponent :
    forall name specializes after_name generic after_generic nodes rest,
      parses_specializes after_name specializes after_generic ->
      parses_generic after_generic generic (TLbrace :: rest) ->
      parses_requires rest nodes (TRbrace :: after_name) ->
      parses_component (TComponent :: TIdentifier name :: after_name)
                       (Component name specializes generic (Seq nodes))
                       after_name.

Inductive parses_imports : list token -> list ast -> list token -> Prop :=
| ParsesImportsStop :
    forall input,
      parses_imports input [] input
| ParsesImportCons :
    forall input path after_path imports rest,
      parses_path input path after_path ->
      parses_imports after_path imports rest ->
      parses_imports (TImport :: input) (Import path :: imports) rest.

Inductive parses_namespace : list token -> ast -> list token -> Prop :=
| ParsesNamespace :
    forall input imports after_imports path after_path component rest,
      parses_imports input imports (TNamespace :: after_imports) ->
      parses_path after_imports path (TLbrace :: after_path) ->
      parses_component after_path component (TRbrace :: rest) ->
      parses_namespace input
        (Seq (imports ++ [Namespace path component]))
        rest.

Lemma parses_path_tail_nonempty :
  forall acc input path rest,
    nonempty_path acc ->
    parses_path_tail acc input path rest ->
    nonempty_path path.
Proof.
  intros acc input path rest Hacc Hparse.
  induction Hparse; auto using nonempty_path_app.
Qed.

Lemma parses_path_nonempty :
  forall input path rest,
    parses_path input path rest ->
    nonempty_path path.
Proof.
  intros input path rest Hparse.
  inversion Hparse; subst.
  eauto using parses_path_tail_nonempty, nonempty_path_single.
Qed.

Lemma parses_source_sound :
  forall input source rest,
    parses_source input source rest -> source_ok source.
Proof.
  intros input source rest Hparse.
  destruct Hparse.
  - left. reflexivity.
  - right. repeat eexists.
Qed.

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

Lemma parses_parts_sound :
  forall input parts rest,
    parses_parts input parts rest ->
    Forall grammar_ast parts /\ Forall component_item_kind parts.
Proof.
  intros input parts rest Hparse.
  induction Hparse.
  - split; constructor.
  - destruct (parses_binds_sound after_body binds (TRbrace :: after_type) H0)
      as [HbindGrammar HbindKind].
    destruct IHHparse as [HpartGrammar HpartKind].
    split; constructor; simpl; auto using GrammarPart.
Qed.

Lemma parses_provide_entries_sound :
  forall input entries rest,
    parses_provide_entries input entries rest ->
    Forall grammar_ast entries /\
    Forall component_item_kind entries /\
    contains_provides entries = true.
Proof.
  intros input entries rest Hparse.
  induction Hparse.
  - assert (Hsource : source_ok source) by eauto using parses_source_sound.
    split.
    + constructor.
      * apply GrammarProvides; exact Hsource.
      * constructor.
    + split.
      * constructor.
        -- simpl. exact I.
        -- constructor.
      * reflexivity.
  - assert (Hsource : source_ok source) by eauto using parses_source_sound.
    destruct IHHparse as [Hgrammar [Hkind Hcontains]].
    split.
    + constructor.
      * apply GrammarProvides; exact Hsource.
      * exact Hgrammar.
    + split.
      * constructor.
        -- simpl. exact I.
        -- exact Hkind.
      * reflexivity.
Qed.

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

Theorem parses_component_sound :
  forall input component rest,
    parses_component input component rest ->
    grammar_ast component /\ ast_wf component /\ is_component component.
Proof.
  intros input component rest Hparse.
  destruct Hparse as
    [name specializes after_name generic after_generic nodes body
     Hspecializes Hgeneric Hrequires].
  destruct (parses_requires_sound body nodes (TRbrace :: after_name) Hrequires)
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

Theorem parses_namespace_sound :
  forall input program rest,
    parses_namespace input program rest ->
    grammar_ast program /\ ast_wf program /\ program_wf program.
Proof.
  intros input program rest Hparse.
  destruct Hparse as
    [input imports after_imports path after_path component rest
     Himports HpathParse HcomponentParse].
  destruct (parses_imports_sound input imports (TNamespace :: after_imports) Himports)
    as [HimportsGrammar [HimportsWf HimportsKind]].
  assert (Hpath : nonempty_path path)
    by eauto using parses_path_nonempty.
  destruct (parses_component_sound after_path component (TRbrace :: rest) HcomponentParse)
    as [HcomponentGrammar [HcomponentWf HcomponentKind]].
  assert (HnamespaceGrammar : grammar_ast (Namespace path component)).
  { apply GrammarNamespace; auto. }
  assert (HnamespaceWf : ast_wf (Namespace path component)).
  { apply grammar_ast_wf; exact HnamespaceGrammar. }
  split.
  - apply GrammarSeq. apply Forall_app_intro.
    + exact HimportsGrammar.
    + constructor.
      * exact HnamespaceGrammar.
      * constructor.
  - split.
    + apply WfSeq. apply Forall_app_intro.
      * exact HimportsWf.
      * constructor.
        -- exact HnamespaceWf.
        -- constructor.
    + simpl. split.
      * apply Forall_app_intro.
        -- exact HimportsWf.
        -- constructor.
           ++ exact HnamespaceWf.
           ++ constructor.
      * apply root_items_imports_namespace; auto.
        simpl. exact I.
Qed.

End MayRustGrammar.
