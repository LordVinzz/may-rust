from __future__ import annotations

import unittest

from generated_path import configure_generated_imports


configure_generated_imports()

from ex1.Start import Start  # noqa: E402
from ex1.client.Client import Client  # noqa: E402
from implementations import (  # noqa: E402
    ClientImpl,
    CompTraceurImpl,
    CompositeImpl,
    CypherImpl,
    MultiClientImpl,
    MultiSimpleImpl,
)


class GeneratedComponentsTest(unittest.TestCase):
    def test_generated_component_is_abstract(self) -> None:
        with self.assertRaises(TypeError):
            Client()

    def test_unbound_requirement_is_reported(self) -> None:
        action = ClientImpl().letsgo()
        with self.assertRaisesRegex(RuntimeError, "demarreur.*not bound"):
            action()

    def test_composite_binding_delegation_and_cache(self) -> None:
        component = CompositeImpl()
        action = component.service()

        self.assertEqual(action(), "composite.simple: call 1")
        self.assertEqual(action(), "composite.simple: call 2")
        self.assertIs(action, component.service())
        self.assertIs(component.parts().simple(), component.parts().simple())
        self.assertEqual(component.parts().simple().created_services, 1)

    def test_traceur_decorates_bound_service(self) -> None:
        component = CompTraceurImpl()

        self.assertEqual(component.service()(), "trace.simple: call 1")
        self.assertEqual(
            component.parts().traceur().events,
            ["before trace.simple", "after trace.simple"],
        )

    def test_specialization_and_codec_bindings(self) -> None:
        source = Start("cypher.input")
        component = CypherImpl(source)

        self.assertIs(component.demarreur(), source)
        self.assertIs(component.parts().codeur().requires().message(), source)
        self.assertIs(component.parts().decodeur().requires().message(), source)

    def test_multiple_clients_share_one_service(self) -> None:
        component = MultiClientImpl()

        self.assertEqual(component.service1()(), "multi-client.simple: call 1")
        self.assertEqual(component.service2()(), "multi-client.simple: call 2")
        self.assertEqual(component.service3()(), "multi-client.simple: call 3")
        self.assertEqual(component.parts().simple().created_services, 1)

    def test_connector_fans_out_to_three_services(self) -> None:
        component = MultiSimpleImpl()

        self.assertEqual(
            component.service()(),
            [
                "simple-1: call 1",
                "simple-2: call 1",
                "simple-3: call 1",
            ],
        )


if __name__ == "__main__":
    unittest.main()
