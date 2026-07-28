from __future__ import annotations

from generated_path import configure_generated_imports


configure_generated_imports()

from ex1.Start import Start  # noqa: E402
from implementations import (  # noqa: E402
    CompTraceurImpl,
    CompositeImpl,
    CypherImpl,
    MultiClientImpl,
    MultiSimpleImpl,
)


def run_composite() -> None:
    component = CompositeImpl()
    action = component.service()
    print("Composite:", action())
    print("Composite:", action())


def run_traced_composite() -> None:
    component = CompTraceurImpl()
    print("CompTraceur:", component.service()())
    print("Trace:", component.parts().traceur().events)


def run_cypher() -> None:
    original = Start("cypher.input")
    component = CypherImpl(original)
    decoded = component.demarreur()
    print("Cypher:", decoded.go())
    assert decoded is original


def run_multi_client() -> None:
    component = MultiClientImpl()
    results = [
        component.service1()(),
        component.service2()(),
        component.service3()(),
    ]
    print("MultiClient:", results)


def run_multi_simple() -> None:
    component = MultiSimpleImpl()
    results = component.service()()
    print("MultiSimple:", results)


def main() -> None:
    run_composite()
    run_traced_composite()
    run_cypher()
    run_multi_client()
    run_multi_simple()


if __name__ == "__main__":
    main()
