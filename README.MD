# MAY Rust

MAY, for Make Agents Yourself, is a project from IRIT focused on defining and generating agent-oriented systems. The original project is available at <https://redmine.irit.fr/projects/may>.

This repository is a Rust redesign of MAY. The goal is to keep the language, tooling, and generated code easier to maintain while improving the reliability of code production. It parses MAY/SpeADL source files into a structured AST and can generate Python or Java components. Formal proof artifacts document key grammar and AST invariants.

## Generation

Run the generator from `may_rust`. The target must be selected explicitly:

```sh
cargo run -- --target python -i ../speadl/ex1 -o ../output/python --keep-intermediate
cargo run -- --target java   -i ../speadl/ex1 -o ../output/java   --keep-intermediate
```

`--target` (or `-t`) is required and accepts `python` and `java`. At least one
`--input` and one `--output` are also required. Omitting any of these arguments
prints the complete CLI help. The same help is available with `-h` or `--help`.
`--keep-intermediate` preserves backend-specific intermediate code:

- Python writes `ComponentName.python_ast_unparse.py`;
- Java writes `ComponentName.java_model.txt`.

Inputs and outputs are provided with `-i`/`--input` and `-o`/`--output`. Both
flags are repeatable. Input directories are expanded recursively and
alphabetically. If one output is provided for multiple inputs, it is treated as
an output directory; repeated outputs are matched to the resulting input order.

The generated Python follows MAY's component implementation model:

- the generated component class is abstract;
- local provided services expose an abstract `make_<service>()` factory;
- parts expose an abstract `make_<part>()` factory;
- `requires()` and `parts()` provide typed access to connected ports and parts;
- part creation, required-port bindings and delegated services are handled by generated code;
- services and parts are initialized lazily and cached.

A concrete application class extends the generated component and implements only
the relevant factories:

```python
class CompositeImpl(Composite):
    def make_simple(self) -> Simple:
        return SimpleImpl()

    def make_client(self) -> Client:
        return ClientImpl()
```

## Java model

The Java backend follows MAY's original JVM model:

- the generated top-level class is the implementation base extended by application code;
- `Requires`, `Provides`, `Parts`, and `Component` are generated as typed nested interfaces;
- local provided ports and parts expose protected abstract `make_<name>()` factories;
- `ComponentImpl` creates parts, installs binding bridges, initializes provided ports, and starts the component tree;
- delegated provided ports call the bound part directly;
- specialization extends the parent component model, including overridden delegated ports;
- SpeADL generic brackets such as `Codec[Start]` become Java generics such as `Codec<Start>`.

Application code supplies only the Java-specific factories:

```java
public final class SimpleImpl extends Simple {
    @Override
    protected Start make_starter() {
        return () -> System.out.println("started");
    }
}

Simple.Component component = new SimpleImpl().newComponent();
component.starter().go();
```

The generated lifecycle and binding structure is based on the official
[MAY JVM model inferrer](https://github.com/victornoel/may/blob/master/fr.irit.smac.may.speadl/src/fr/irit/smac/may/speadl/jvmmodel/SpeADLJvmModelInferrer.xtend)
and the [MAY SpeADL reference](https://redmine.irit.fr/projects/may/wiki/SpeADL_Minus_Java_Reference).
