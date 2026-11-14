"""Independently authored synthetic tables for read-only offer behavior.

Column compatibility reference: BitLupa ead5e0f6caa63db9a3bf640f0e8652df6548a25f,
https://github.com/AlexandreZanata/BitLupa/blob/ead5e0f6caa63db9a3bf640f0e8652df6548a25f/db/migrations/0001_init.sql
No source migration, private data, collector or live network access is reproduced.
"""
import contextlib
import hashlib
import importlib.util
import io
import json
from pathlib import Path
import sqlite3
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("bitlupa_service", Path(__file__).with_name("bitlupa_service.py"))
bridge = importlib.util.module_from_spec(spec)
spec.loader.exec_module(bridge)

SCHEMA = "\nCREATE TABLE sources (id TEXT PRIMARY KEY, status TEXT);\nCREATE TABLE products (id INTEGER PRIMARY KEY, category TEXT, brand TEXT, model TEXT, storage_gb INTEGER, ram_gb INTEGER);\nCREATE TABLE raw_documents (id INTEGER PRIMARY KEY, source_id TEXT, url TEXT, fetched_at INTEGER, sha256 TEXT);\nCREATE TABLE source_items (id INTEGER PRIMARY KEY, source_id TEXT, raw_document_id INTEGER, parse_status TEXT);\nCREATE TABLE source_products (source_item_id INTEGER PRIMARY KEY, product_id INTEGER, match_method TEXT, match_confidence REAL);\nCREATE TABLE offers (id INTEGER PRIMARY KEY, product_id INTEGER, source_id TEXT, source_item_id INTEGER,\n condition TEXT, price_cents INTEGER, price_kind TEXT, currency TEXT DEFAULT 'BRL', availability TEXT,\n url TEXT, collected_at INTEGER, valid_until INTEGER);\n"

class BitLupaContracts(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="zaturo-bitlupa-fixture-")
        self.root = Path(self.temporary.name)
        (self.root / "config/categories").mkdir(parents=True)
        self.now = 1_800_000_000
        self.source = {"id": "kabum", "name": "KaBuM!", "kind": "retailer", "website": "https://www.kabum.com.br",
                       "phase": 1, "in_scope": True, "enabled": True, "status": "enabled",
                       "access": {"robots_reviewed": True, "terms_reviewed": True, "evidence_ref": "research/kabum/EVIDENCE.md"}}
        self.targets = [{"id": "xiaomi-poco-x7-pro-512-12", "brand": "Xiaomi", "model": "POCO X7 Pro", "storage_gb": 512, "ram_gb": 12},
                        {"id": "xiaomi-poco-x7-pro-256-8", "brand": "Xiaomi", "model": "POCO X7 Pro", "storage_gb": 256, "ram_gb": 8}]
        self.config()
        self.database = self.root / "observations.db"
        self.db = sqlite3.connect(self.database)
        self.db.executescript(SCHEMA)
        self.db.execute("INSERT INTO sources VALUES (?,?)", ("kabum", "enabled"))
        self.product(1, "POCO X7 Pro", 512, 12)
        self.product(2, "POCO X7 Pro", 256, 8)
        self.product(3, "POCO X7", 512, 12)
        self.db.commit()

    def tearDown(self):
        self.db.close()
        self.temporary.cleanup()

    def config(self):
        (self.root / "config/sources.json").write_text(json.dumps({"category": "cellphones", "sources": [self.source]}))
        (self.root / "config/categories/cellphones.json").write_text(json.dumps({"condition_scope": ["new"], "tracked_products": self.targets}))

    def product(self, identity, model, storage, ram):
        self.db.execute("INSERT INTO products VALUES (?,?,?,?,?,?)", (identity, "cellphones", "Xiaomi", model, storage, ram))

    def offer(self, identity, price, *, product=1, age=60, condition="new", availability="in_stock", price_kind="pix", url=None, confidence=.95, expiry=None):
        collected = self.now - age
        url = url or "https://www.kabum.com.br/produto/synthetic-" + str(identity)
        self.db.execute("INSERT INTO raw_documents VALUES (?,?,?,?,?)", (identity, "kabum", url, collected - 1, format(identity, "064x")))
        self.db.execute("INSERT INTO source_items VALUES (?,?,?,?)", (identity, "kabum", identity, "parsed"))
        self.db.execute("INSERT INTO source_products VALUES (?,?,?,?)", (identity, product, "identity", confidence))
        self.db.execute("INSERT INTO offers (id,product_id,source_id,source_item_id,condition,price_cents,price_kind,availability,url,collected_at,valid_until) VALUES (?,?,?,?,?,?,?,?,?,?,?)",
                        (identity, product, "kabum", identity, condition, price, price_kind, availability, url, collected, expiry))
        self.db.commit()

    def search(self, query="POCO X7 Pro 512GB", limit=5):
        return bridge.search(self.root, self.database, query, self.now, limit)

    def test_exact_variant_never_replaces_x7_with_pro_or_different_capacity(self):
        self.offer(1, 210000)
        self.offer(2, 170000, product=2)
        self.offer(3, 160000, product=3)
        self.assertEqual(self.search("Poco X7")["code"], "untracked_product")
        self.assertEqual([o["id"] for o in self.search()["offers"]], [1])
        self.assertEqual([o["id"] for o in self.search("Xiaomi POCO X7 Pro 256 GB 8GB RAM")["offers"]], [2])
        self.assertEqual(self.search("POCO X7 Ultra")["code"], "untracked_product")

    def test_only_observed_authorized_new_exact_offers_are_ranked_with_freshness(self):
        self.offer(1, 210000)
        self.offer(2, 180000, age=30000)
        self.offer(3, 150000, condition="used")
        self.offer(4, 140000, confidence=.8)
        self.offer(5, 200000, availability="out_of_stock")
        self.offer(6, 120000, url="https://evil.invalid/exfil")
        result = self.search()
        self.assertEqual(result["lowest_observed_offer_id"], 1)
        self.assertIsNone(result["best_value_offer_id"])
        self.assertEqual([o["id"] for o in result["offers"]], [1, 5, 2])
        self.assertEqual(result["offers"][-1]["freshness"], "stale")
        self.assertEqual(result["offers"][0]["currency"], "BRL")
        self.assertTrue(result["offers"][0]["collected_at"].endswith("Z"))
        self.assertFalse(result["live_collection"])
        self.assertIn("Frete", result["comparison_note_pt"])

    def test_disabled_unreviewed_and_query_credential_urls_never_become_offers(self):
        self.offer(1, 210000, url="https://user:secret@www.kabum.com.br/produto/one")
        self.offer(2, 200000, url="https://www.kabum.com.br/produto/two?token=synthetic")
        self.assertFalse(self.search()["offers"])
        self.source["access"]["terms_reviewed"] = False
        self.config()
        self.assertEqual(self.search()["code"], "bitlupa_sources_unavailable")
        self.source["access"]["terms_reviewed"] = True
        self.source["enabled"] = False
        self.config()
        self.assertEqual(self.search()["code"], "bitlupa_sources_unavailable")
        self.source["enabled"] = True
        self.source["website"] = "http://127.0.0.1:7878"
        self.config()
        self.assertEqual(self.search()["code"], "bitlupa_sources_unavailable")

    def test_expired_unknown_or_installment_observations_do_not_claim_best_price(self):
        self.offer(1, 210000, expiry=self.now - 1)
        self.offer(2, 200000, price_kind="unknown")
        self.offer(3, 190000, price_kind="installment_total")
        result = self.search()
        self.assertEqual(len(result["offers"]), 3)
        self.assertIsNone(result["lowest_observed_offer_id"])
        self.assertEqual(result["status"], "partial")

    def test_database_is_read_only_bounded_and_missing_schema_is_honest(self):
        self.offer(1, 210000)
        before = hashlib.sha256(self.database.read_bytes()).hexdigest()
        self.assertEqual(self.search()["status"], "completed")
        self.assertEqual(before, hashlib.sha256(self.database.read_bytes()).hexdigest())
        symlink = self.root / "redirect.db"
        symlink.symlink_to(self.database)
        result = bridge.search(self.root, symlink, "POCO X7 Pro", self.now)
        self.assertEqual(result["code"], "bitlupa_database_unavailable")
        result = bridge.search(self.root, self.root / "absent.db", "POCO X7 Pro", self.now)
        self.assertEqual(result["code"], "bitlupa_database_missing")
        self.assertFalse((self.root / "absent.db").exists())
        self.db.execute("DROP TABLE offers")
        self.db.commit()
        self.assertEqual(self.search()["code"], "bitlupa_database_unavailable")

    def test_oversized_database_rows_and_recent_candidate_limit_fail_bounded(self):
        for identity in range(1, bridge.MAX_CANDIDATES + 2):
            self.offer(identity, 210000 + identity)
        result = self.search(limit=3)
        self.assertEqual(len(result["offers"]), 3)
        self.assertTrue(result["coverage_limited"])
        self.assertEqual(result["status"], "partial")
        self.assertEqual(result["ranking_scope"], "displayed_observations_only")
        self.db.execute("UPDATE offers SET url=?", ("https://www.kabum.com.br/" + "a" * 70000,))
        self.db.commit()
        result = self.search()
        self.assertEqual(result["code"], "bitlupa_database_unavailable")
        self.assertLess(len(json.dumps(result).encode()), bridge.MAX_RESPONSE_BYTES)

    def test_cli_refuses_secret_queries_and_does_not_echo_private_paths_or_exceptions(self):
        stream = io.StringIO()
        with contextlib.redirect_stdout(stream):
            self.assertEqual(bridge.main(["--project-root", str(self.root), "search", "--query", ("oc_" + "sk_" + "synthetic-never-echo"), "--now", str(self.now)]), 0)
        self.assertNotIn(("oc_" + "sk_" + "synthetic"), stream.getvalue())
        self.assertNotIn(str(self.root), stream.getvalue())
        self.assertEqual(json.loads(stream.getvalue())["code"], "invalid_query")
        stream = io.StringIO()
        with contextlib.redirect_stdout(stream):
            bridge.main(["--project-root", str(self.root), "capabilities"])
        result = json.loads(stream.getvalue())
        self.assertEqual(result["category"], "cellphones")
        self.assertEqual(result["tracked_models"], ["POCO X7 Pro"])
        self.assertEqual(result["sources"][0]["id"], "kabum")
        self.assertTrue(result["readonly"])


if __name__ == "__main__":
    unittest.main()
