From Stdlib Require Import Arith.PeanoNat.
From Stdlib Require Import Bool.Bool.
From Stdlib Require Import Lia.
From Stdlib Require Import Lists.List.
From Stdlib Require Import Strings.Ascii.
From Stdlib Require Import Strings.String.
From MayRustProofs Require Import Grammar.

Import ListNotations.
Import MayRustGrammar.

(**
 * @brief Regroupe le lexer de référence et les preuves de correction associées.
 *)
Module LexerCorrectness.

Open Scope char_scope.

(**
 * @brief Convertit récursivement une chaîne Coq en liste de caractères ASCII.
 * @param source Chaîne à décomposer.
 * @return Liste des caractères de [source], dans leur ordre d'origine.
 *)
Fixpoint chars_of_string (source : string) : list ascii :=
  match source with
  | EmptyString => []
  | String character rest => character :: chars_of_string rest
  end.

(**
 * @brief Reconstruit récursivement une chaîne Coq depuis des caractères ASCII.
 * @param characters Caractères à concaténer.
 * @return Chaîne contenant les caractères fournis dans le même ordre.
 *)
Fixpoint string_of_chars (characters : list ascii) : string :=
  match characters with
  | [] => EmptyString
  | character :: rest => String character (string_of_chars rest)
  end.

(**
 * @brief Teste si un entier naturel appartient à un intervalle fermé.
 * @param lower Borne inférieure inclusive.
 * @param value Valeur à tester.
 * @param upper Borne supérieure inclusive.
 * @return [true] exactement lorsque [lower <= value <= upper].
 *)
Definition between (lower value upper : nat) : bool :=
  Nat.leb lower value && Nat.leb value upper.

(**
 * @brief Reproduit [char::is_whitespace] de Rust pour les caractères ASCII.
 * @param character Caractère dont la nature doit être testée.
 * @return [true] si [character] est un espace ASCII reconnu par le lexer.
 *)
Definition is_ascii_whitespace (character : ascii) : bool :=
  match nat_of_ascii character with
  | 9 | 10 | 11 | 12 | 13 | 32 => true
  | _ => false
  end.

(**
 * @brief Reconnaît exactement la classe utilisée par [CharReader::read_identifier].
 * @param character Caractère à tester.
 * @return [true] pour une lettre ASCII, un chiffre ou un trait de soulignement.
 *)
Definition is_identifier_character (character : ascii) : bool :=
  let code := nat_of_ascii character in
  between 97 code 122 ||
  between 65 code 90 ||
  Ascii.eqb character "_"%char ||
  between 48 code 57.

(**
 * @brief Supprime récursivement les espaces ASCII situés en tête d'une entrée.
 * @param input Suite de caractères à examiner.
 * @return Suffixe commençant au premier caractère non blanc, ou la liste vide.
 *)
Fixpoint skip_whitespace (input : list ascii) : list ascii :=
  match input with
  | character :: rest =>
      if is_ascii_whitespace character
      then skip_whitespace rest
      else input
  | [] => []
  end.

(**
 * @brief Lit le plus long préfixe composé de caractères d'identificateur.
 * @param input Suite de caractères à analyser.
 * @return Couple formé de l'identificateur lu et du suffixe non consommé.
 *)
Fixpoint read_identifier (input : list ascii) : list ascii * list ascii :=
  match input with
  | character :: rest =>
      if is_identifier_character character
      then
        let '(identifier, after_identifier) := read_identifier rest in
        (character :: identifier, after_identifier)
      else ([], input)
  | [] => ([], [])
  end.

(**
 * @brief Associe à un caractère de ponctuation le token correspondant.
 * @param character Caractère éventuellement reconnu comme ponctuation SPEADL.
 * @return Le token associé, ou [None] si le caractère n'est pas une ponctuation.
 *)
Definition punctuation_token (character : ascii) : option token :=
  if Ascii.eqb character "."%char then Some TDot else
  if Ascii.eqb character ":"%char then Some TColon else
  if Ascii.eqb character "="%char then Some TEquals else
  if Ascii.eqb character "{"%char then Some TLbrace else
  if Ascii.eqb character "}"%char then Some TRbrace else
  if Ascii.eqb character "["%char then Some TLbracket else
  if Ascii.eqb character "]"%char then Some TRbracket else
  None.

(**
 * @brief Classe un lexème textuel comme mot-clé réservé ou identificateur.
 * @param identifier Lexème complet à classifier.
 * @return Le token de mot-clé correspondant, sinon [TIdentifier identifier].
 *)
Definition keyword_or_identifier (identifier : string) : token :=
  if String.eqb identifier "import"%string then TImport else
  if String.eqb identifier "namespace"%string then TNamespace else
  if String.eqb identifier "component"%string then TComponent else
  if String.eqb identifier "specializes"%string then TSpecializes else
  if String.eqb identifier "provides"%string then TProvides else
  if String.eqb identifier "requires"%string then TRequires else
  if String.eqb identifier "part"%string then TPart else
  if String.eqb identifier "bind"%string then TBind else
  if String.eqb identifier "to"%string then TTo else
  TIdentifier identifier.

(**
 * @brief Décrit les trois résultats possibles de l'exécution du lexer.
 *)
Inductive lexer_outcome : Type :=
| LexerAccepted : list token -> lexer_outcome
| LexerInvalidCharacter : ascii -> lexer_outcome
| LexerOutOfFuel : lexer_outcome.

(**
 * @brief Ajoute un token devant un résultat accepté sans masquer les erreurs.
 * @param current Token à ajouter.
 * @param outcome Résultat de la lexémisation du suffixe.
 * @return Résultat enrichi si accepté, ou erreur initiale inchangée.
 *)
Definition prepend_token (current : token) (outcome : lexer_outcome)
    : lexer_outcome :=
  match outcome with
  | LexerAccepted tokens => LexerAccepted (current :: tokens)
  | LexerInvalidCharacter character => LexerInvalidCharacter character
  | LexerOutOfFuel => LexerOutOfFuel
  end.

(**
 * @brief Prouve que l'ajout d'un token ne transforme pas un résultat disponible en épuisement de carburant.
 * @param current Token à ajouter au résultat.
 * @param outcome Résultat dont la disponibilité du carburant est connue.
 * @return Preuve que [prepend_token current outcome] n'est pas [LexerOutOfFuel].
 *)
Lemma prepend_token_preserves_available_fuel :
  forall current outcome,
    outcome <> LexerOutOfFuel ->
    prepend_token current outcome <> LexerOutOfFuel.
Proof.
  intros current outcome Havailable.
  destruct outcome; simpl; try discriminate.
  contradiction.
Qed.

(**
 * @brief Lexe une entrée ASCII avec une borne explicite sur les étapes récursives.
 * @param fuel Nombre maximal d'étapes de lexémisation disponibles.
 * @param input Caractères restant à analyser.
 * @return Tokens acceptés, caractère invalide ou signal d'épuisement du carburant.
 *)
Fixpoint lex_ascii_with_fuel (fuel : nat) (input : list ascii)
    : lexer_outcome :=
  match fuel with
  | 0 => LexerOutOfFuel
  | S remaining_fuel =>
      match skip_whitespace input with
      | [] => LexerAccepted [TEOF]
      | character :: rest =>
          match punctuation_token character with
          | Some current =>
              prepend_token current
                (lex_ascii_with_fuel remaining_fuel rest)
          | None =>
              if is_identifier_character character
              then
                let '(identifier, after_identifier) :=
                  read_identifier (character :: rest) in
                prepend_token
                  (keyword_or_identifier (string_of_chars identifier))
                  (lex_ascii_with_fuel remaining_fuel after_identifier)
              else LexerInvalidCharacter character
          end
      end
  end.

(**
 * @brief Lexe une liste ASCII avec un carburant strictement supérieur à sa longueur.
 * @param input Caractères à analyser.
 * @return Résultat complet du lexer de référence.
 *)
Definition lex_ascii (input : list ascii) : lexer_outcome :=
  lex_ascii_with_fuel (S (List.length input)) input.

(**
 * @brief Lexe directement une chaîne Coq au moyen du lexer ASCII de référence.
 * @param source Texte source à analyser.
 * @return Résultat du lexer appliqué aux caractères de [source].
 *)
Definition lex_string (source : string) : lexer_outcome :=
  lex_ascii (chars_of_string source).

(**
 * @brief Exprime que tous les caractères d'une source appartiennent à l'ASCII sur 7 bits.
 * @param input Caractères composant la source.
 * @return Proposition imposant un code strictement inférieur à 128 à chaque caractère.
 *)
Definition ascii_source (input : list ascii) : Prop :=
  Forall (fun character => Nat.lt (nat_of_ascii character) 128) input.

(**
 * @brief Prouve que la suppression des espaces de tête n'allonge jamais l'entrée.
 * @param input Suite de caractères à simplifier.
 * @return Preuve que le suffixe obtenu n'est pas plus long que [input].
 *)
Lemma skip_whitespace_does_not_grow :
  forall input,
    Nat.le (List.length (skip_whitespace input)) (List.length input).
Proof.
  induction input as [|character rest IH].
  - simpl. apply Nat.le_refl.
  - simpl. destruct (is_ascii_whitespace character); simpl; lia.
Qed.

(**
 * @brief Prouve que le suffixe laissé par la lecture d'un identificateur n'est pas plus long que l'entrée.
 * @param input Suite analysée.
 * @param identifier Préfixe reconnu comme identificateur.
 * @param after_identifier Suffixe laissé par la lecture.
 * @return Preuve de la borne de longueur sur [after_identifier].
 *)
Lemma read_identifier_remainder_does_not_grow :
  forall input identifier after_identifier,
    read_identifier input = (identifier, after_identifier) ->
    Nat.le (List.length after_identifier) (List.length input).
Proof.
  induction input as [|character rest IH];
    intros identifier after_identifier Hread.
  - simpl in Hread. inversion Hread. apply Nat.le_refl.
  - simpl in Hread.
    destruct (is_identifier_character character) eqn:Hidentifier.
    + destruct (read_identifier rest)
        as [rest_identifier rest_after] eqn:Hrest.
      pose proof (IH rest_identifier rest_after eq_refl) as Hbound.
      inversion Hread; subst.
      simpl. lia.
    + inversion Hread; subst. simpl. lia.
Qed.

(**
 * @brief Prouve qu'un premier caractère valide est effectivement consommé par la lecture d'identificateur.
 * @param character Premier caractère de l'entrée.
 * @param rest Suffixe suivant ce caractère.
 * @param identifier Identificateur produit.
 * @param after_identifier Suffixe non consommé.
 * @return Preuve que [after_identifier] est strictement plus court que l'entrée initiale.
 *)
Lemma read_identifier_consumes_its_first_character :
  forall character rest identifier after_identifier,
    is_identifier_character character = true ->
    read_identifier (character :: rest) = (identifier, after_identifier) ->
    Nat.lt (List.length after_identifier)
           (List.length (character :: rest)).
Proof.
  intros character rest identifier after_identifier Hidentifier Hread.
  simpl in Hread. rewrite Hidentifier in Hread.
  destruct (read_identifier rest)
    as [rest_identifier rest_after] eqn:Hrest.
  pose proof (read_identifier_remainder_does_not_grow
                rest rest_identifier rest_after Hrest) as Hbound.
  inversion Hread; subst.
  simpl. lia.
Qed.

(**
 * @brief Établit qu'un carburant supérieur à la longueur d'entrée suffit toujours au lexer.
 * @param fuel Carburant fourni à l'exécution.
 * @param input Caractères à analyser.
 * @return Preuve que le résultat ne peut pas être [LexerOutOfFuel].
 *)
Theorem lex_ascii_with_sufficient_fuel_never_runs_out :
  forall fuel input,
    Nat.lt (List.length input) fuel ->
    lex_ascii_with_fuel fuel input <> LexerOutOfFuel.
Proof.
  induction fuel as [|remaining_fuel IH]; intros input Hlength.
  - simpl in Hlength. lia.
  - simpl.
    destruct (skip_whitespace input) as [|character rest] eqn:Hskip.
    + discriminate.
    + assert (HrestInput :
        Nat.lt (List.length rest) (List.length input)).
      { pose proof (skip_whitespace_does_not_grow input) as HskipLength.
        rewrite Hskip in HskipLength. simpl in HskipLength. lia. }
      destruct (punctuation_token character) as [current|] eqn:Hpunctuation.
      * apply prepend_token_preserves_available_fuel.
        apply IH. lia.
      * destruct (is_identifier_character character) eqn:Hidentifier.
        -- destruct (read_identifier rest)
             as [identifier after_identifier] eqn:Hread.
           pose proof (read_identifier_remainder_does_not_grow
                         rest identifier after_identifier Hread) as HafterRest.
           apply prepend_token_preserves_available_fuel.
           apply IH. lia.
        -- discriminate.
Qed.

(**
 * @brief Établit que le lexer ASCII public ne manque jamais de carburant.
 * @param input Caractères à analyser.
 * @return Preuve que [lex_ascii input] n'est jamais [LexerOutOfFuel].
 *)
Theorem lex_ascii_never_runs_out_of_fuel :
  forall input, lex_ascii input <> LexerOutOfFuel.
Proof.
  intros input.
  apply lex_ascii_with_sufficient_fuel_never_runs_out.
  simpl. lia.
Qed.

(**
 * @brief Représente une observation exécutable du lexer Rust natif.
 * @return Fonction qui associe une entrée ASCII au résultat observé du lexer.
 *)
Definition rust_lexer_runner : Type := list ascii -> lexer_outcome.

(**
 * @brief Exprime l'égalité du lexer Rust observé et du lexer de référence sur toute source ASCII.
 * @param run Observation du lexer Rust natif à comparer.
 * @return Proposition d'exactitude point par point sur les entrées ASCII.
 *)
Definition rust_lexer_exact_on_ascii (run : rust_lexer_runner) : Prop :=
  forall input,
    ascii_source input ->
    run input = lex_ascii input.

(**
 * @brief Déduit qu'une suite de tokens acceptée par un lexer Rust exact est aussi produite par la référence.
 * @param run Observation du lexer Rust natif.
 * @param input Entrée ASCII analysée.
 * @param tokens Tokens acceptés par [run].
 * @return Preuve que [lex_ascii input] accepte exactement [tokens].
 *)
Theorem exact_rust_lexer_produces_reference_tokens :
  forall run input tokens,
    rust_lexer_exact_on_ascii run ->
    ascii_source input ->
    run input = LexerAccepted tokens ->
    lex_ascii input = LexerAccepted tokens.
Proof.
  intros run input tokens Hexact Hascii Hrun.
  rewrite <- (Hexact input Hascii).
  exact Hrun.
Qed.

(**
 * @brief Déduit que les tokens de référence sont aussi produits par un lexer Rust exact.
 * @param run Observation du lexer Rust natif.
 * @param input Entrée ASCII analysée.
 * @param tokens Tokens acceptés par le lexer de référence.
 * @return Preuve que [run input] accepte exactement [tokens].
 *)
Theorem reference_tokens_are_exact_rust_tokens :
  forall run input tokens,
    rust_lexer_exact_on_ascii run ->
    ascii_source input ->
    lex_ascii input = LexerAccepted tokens ->
    run input = LexerAccepted tokens.
Proof.
  intros run input tokens Hexact Hascii Hreference.
  rewrite (Hexact input Hascii).
  exact Hreference.
Qed.

(**
 * @brief Prouve le déterminisme des suites de tokens acceptées par le lexer ASCII.
 * @param input Entrée dont les résultats sont comparés.
 * @param tokens1 Première suite de tokens acceptée.
 * @param tokens2 Seconde suite de tokens acceptée.
 * @return Preuve de l'égalité entre [tokens1] et [tokens2].
 *)
Theorem lex_ascii_is_deterministic :
  forall input tokens1 tokens2,
    lex_ascii input = LexerAccepted tokens1 ->
    lex_ascii input = LexerAccepted tokens2 ->
    tokens1 = tokens2.
Proof.
  intros input tokens1 tokens2 Htokens1 Htokens2.
  rewrite Htokens1 in Htokens2.
  inversion Htokens2. reflexivity.
Qed.

(**
 * @brief Vérifie par calcul la lexémisation d'un composant minimal valide.
 * @return Égalité entre le résultat calculé et la suite de tokens attendue.
 *)
Example lexes_minimal_component_tokens :
  lex_string "namespace demo { component Main { provides service: Service } }"%string =
  LexerAccepted
    [TNamespace; TIdentifier "demo"%string; TLbrace;
     TComponent; TIdentifier "Main"%string; TLbrace;
     TProvides; TIdentifier "service"%string; TColon;
     TIdentifier "Service"%string; TRbrace; TRbrace; TEOF].
Proof.
  reflexivity.
Qed.

(**
 * @brief Vérifie par calcul que le point-virgule est rejeté comme caractère invalide.
 * @return Égalité entre le résultat calculé et l'erreur attendue.
 *)
Example rejects_semicolon_like_the_speadl_lexer :
  lex_string ";"%string = LexerInvalidCharacter ";"%char.
Proof.
  reflexivity.
Qed.

End LexerCorrectness.
