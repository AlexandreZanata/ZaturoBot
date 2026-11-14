#!/usr/bin/env python3
"""Read BitLupa's observed normalized offers; never collect, invent prices or buy.

BitLupa currently publishes a schema/config base, not a runnable collector/API. This
bridge implements an actual read-only query of that schema when its local database
exists. It never presents indexed snippets, historical evidence or fixtures as prices.
Reference contract: https://github.com/AlexandreZanata/BitLupa at
 ead5e0f6caa63db9a3bf640f0e8652df6548a25f, MIT.
No BitLupa runtime is imported: that revision has no collectors, backend or API.
"""
from __future__ import annotations

import argparse
import datetime as dt
import json
import os
from pathlib import Path
import re
import sqlite3
import stat
import time
import unicodedata
from urllib.parse import urlsplit

MAX_CONFIG_BYTES = 256 * 1024
MAX_DATABASE_BYTES = 512 * 1024 * 1024
MAX_RESPONSE_BYTES = 16 * 1024
MAX_CANDIDATES = 200
FRESH_SECONDS = 6 * 60 * 60
RETAILER_HOSTS = {"kabum": "www.kabum.com.br", "pichau": "www.pichau.com.br",
                  "terabyte": "www.terabyteshop.com.br", "gigantec": "www.gigantec.com.br",
                  "waz": "www.waz.com.br"}
RETAILERS = set(RETAILER_HOSTS)


class Rejected(ValueError):
    def __init__(self, code: str):
        self.code = code
        super().__init__(code)


def regular(path: Path, maximum: int) -> Path:
    if not path.is_absolute() or any(p.is_symlink() for p in (path, *path.parents)):
        raise Rejected("unsafe_local_path")
    info = path.stat()
    if not stat.S_ISREG(info.st_mode) or info.st_uid != os.getuid() or info.st_size > maximum:
        raise Rejected("unsafe_local_file")
    return path


def configuration(root: Path) -> tuple[dict, list[dict]]:
    source = regular(root / "config/sources.json", MAX_CONFIG_BYTES)
    category = regular(root / "config/categories/cellphones.json", MAX_CONFIG_BYTES)
    registry = json.loads(source.read_text())
    targets = json.loads(category.read_text())
    if registry.get("category") != "cellphones" or targets.get("condition_scope") != ["new"]:
        raise Rejected("unsupported_bitlupa_contract")
    stores = registry.get("sources")
    products = targets.get("tracked_products")
    if not isinstance(stores, list) or not 1 <= len(stores) <= 16:
        raise Rejected("invalid_bitlupa_registry")
    if not isinstance(products, list) or not 1 <= len(products) <= 100:
        raise Rejected("invalid_bitlupa_targets")
    if any(not isinstance(s, dict) or not isinstance(s.get("id"), str)
           or not isinstance(s.get("name"), str) or len(s["name"].encode()) > 64
           for s in stores) or len({s["id"] for s in stores}) != len(stores):
        raise Rejected("invalid_bitlupa_registry")
    for product in products:
        if (not isinstance(product, dict) or not isinstance(product.get("brand"), str)
                or not isinstance(product.get("model"), str) or len(product["brand"].encode()) > 64
                or len(product["model"].encode()) > 80 or type(product.get("storage_gb")) is not int
                or not 1 <= product["storage_gb"] <= 4096
                or (product.get("ram_gb") is not None and (type(product["ram_gb"]) is not int or not 1 <= product["ram_gb"] <= 128))):
            raise Rejected("invalid_bitlupa_targets")
    return {s["id"]: s for s in stores}, products


def normalized(value: str) -> str:
    if not isinstance(value, str):
        raise Rejected("invalid_text")
    value = "".join(c for c in unicodedata.normalize("NFD", value.lower()) if not unicodedata.combining(c))
    value = re.sub(r"(\d+)\s*gb\b", r"\1gb", value)
    return " ".join(re.sub(r"[^a-z0-9]+", " ", value).split())


def selected_targets(query: str, products: list[dict]) -> list[dict]:
    if not isinstance(query, str) or not query.strip() or len(query.encode()) > 160 or any(ord(c) < 32 for c in query):
        raise Rejected("invalid_query")
    if re.search(r"oc_sk_|-----BEGIN|AGE-SECRET|bearer\s|api[_ -]?key", query, re.I):
        raise Rejected("invalid_query")
    words = normalized(query).split()
    selected = []
    for product in products:
        model = normalized(product["model"]).split()
        brand = normalized(product["brand"]).split()
        rest = words[len(brand):] if words[:len(brand)] == brand else words
        # Exact model tokens retain Pro/Plus/Ultra/FE; a base X7 is never changed to X7 Pro.
        if rest[:len(model)] != model:
            continue
        suffix = [word for word in rest[len(model):] if word not in {"ram", "5g"}]
        capacities = {str(product["storage_gb"]) + "gb"}
        if product.get("ram_gb") is not None:
            capacities.add(str(product["ram_gb"]) + "gb")
        if all(word in capacities for word in suffix):
            selected.append(product)
    return selected


def active(source: dict) -> bool:
    access = source.get("access", {})
    return (source.get("id") in RETAILERS and source.get("kind") == "retailer"
            and source.get("phase") == 1 and source.get("in_scope") is True
            and source.get("enabled") is True and source.get("status") in {"enabled", "degraded"}
            and access.get("robots_reviewed") is True and access.get("terms_reviewed") is True
            and isinstance(access.get("evidence_ref"), str) and bool(access["evidence_ref"])
            and source_url(source.get("website", ""), source))


def source_url(value: str, source: dict) -> bool:
    if not isinstance(value, str) or len(value.encode()) > 512 or any(ord(c) < 33 for c in value):
        return False
    try:
        parsed = urlsplit(value)
        allowed = RETAILER_HOSTS.get(source.get("id"))
        return (bool(allowed) and parsed.scheme == "https" and parsed.hostname == allowed and parsed.port in {None, 443}
                and not parsed.username and not parsed.password and not parsed.fragment and not parsed.query)
    except (ValueError, KeyError, TypeError):
        return False


def timestamp(value: int) -> str:
    return dt.datetime.fromtimestamp(value, dt.timezone.utc).isoformat().replace("+00:00", "Z")


def base(query: str, now: int, stores: dict) -> dict:
    return {"schema": 1, "status": "blocked", "code": "not_ready", "query": query,
            "observed_at": timestamp(now), "live_collection": False, "category": "cellphones", "condition_scope": ["new"],
            "summary_pt": "", "offers": [], "lowest_observed_offer_id": None,
            "best_value_offer_id": None,
            "ranking_scope": "displayed_observations_only", "freshness_window_seconds": FRESH_SECONDS,
            "comparison_note_pt": "Frete, garantia e reputação não foram verificados. Não é possível afirmar o melhor custo-benefício total. Preço Pix/à vista e variantes são exibidos separadamente.",
            "sources": [{"id": s["id"], "name": s["name"], "status": s.get("status", "unknown"), "eligible": active(s)}
                        for s in stores.values() if s.get("id") in RETAILERS]}


def read_offers(database: Path, stores: dict, targets: list[dict], now: int) -> tuple[list[dict], bool]:
    regular(database, MAX_DATABASE_BYTES)
    deadline = time.monotonic() + 3
    conn = sqlite3.connect(database.as_uri() + "?mode=ro", uri=True, timeout=1)
    try:
        conn.setlimit(sqlite3.SQLITE_LIMIT_LENGTH, 65536)
        conn.execute("PRAGMA query_only=ON")
        conn.execute("PRAGMA trusted_schema=OFF")
        conn.set_progress_handler(lambda: 1 if time.monotonic() > deadline else 0, 1000)
        # Read only normalized tables, never raw snapshot paths, payloads or credentials.
        conn.row_factory = sqlite3.Row
        clauses, parameters = [], []
        for target in targets:
            clause = "(p.brand=? COLLATE NOCASE AND p.model=? COLLATE NOCASE AND p.storage_gb=?"
            parameters.extend([target["brand"], target["model"], target["storage_gb"]])
            if target.get("ram_gb") is not None:
                clause += " AND p.ram_gb=?"
                parameters.append(target["ram_gb"])
            clauses.append(clause + ")")
        rows = conn.execute("""
          SELECT o.id,o.product_id,o.source_id,o.price_cents,o.price_kind,o.currency,
                 o.availability,o.url,o.collected_at,o.valid_until,p.brand,p.model,
                 p.storage_gb,p.ram_gb,s.status AS source_status,
                 si.parse_status,sp.match_confidence,sp.match_method,rd.sha256 AS raw_sha256,
                 rd.fetched_at,rd.url AS raw_url
          FROM offers o JOIN products p ON p.id=o.product_id
          JOIN sources s ON s.id=o.source_id
          JOIN source_items si ON si.id=o.source_item_id AND si.source_id=o.source_id
          JOIN source_products sp ON sp.source_item_id=si.id AND sp.product_id=p.id
          JOIN raw_documents rd ON rd.id=si.raw_document_id AND rd.source_id=o.source_id
          WHERE p.category='cellphones' AND o.condition='new' AND (""" + " OR ".join(clauses) + """ )
          ORDER BY o.collected_at DESC,o.id DESC LIMIT ?
        """, [*parameters, MAX_CANDIDATES + 1]).fetchall()
        truncated = len(rows) > MAX_CANDIDATES
        offers = []
        for row in rows[:MAX_CANDIDATES]:
            source = stores.get(row["source_id"])
            if not source or not active(source) or row["source_status"] not in {"enabled", "degraded"}:
                continue
            if not any(normalized(row["brand"]) == normalized(t["brand"])
                       and normalized(row["model"]) == normalized(t["model"])
                       and row["storage_gb"] == t["storage_gb"]
                       and (t.get("ram_gb") is None or row["ram_gb"] == t["ram_gb"]) for t in targets):
                continue
            if (row["parse_status"] != "parsed" or row["match_method"] not in {"gtin", "identity", "manual"}
                    or not isinstance(row["match_confidence"], (int, float)) or row["match_confidence"] < .95
                    or not re.fullmatch(r"[a-f0-9]{64}", row["raw_sha256"])
                    or not source_url(row["url"], source) or not source_url(row["raw_url"], source)
                    or row["currency"] != "BRL" or type(row["price_cents"]) is not int
                    or not 30000 <= row["price_cents"] <= 3000000
                    or type(row["collected_at"]) is not int or not 0 <= row["collected_at"] <= now
                    or type(row["fetched_at"]) is not int or not 0 <= row["fetched_at"] <= row["collected_at"]
                    or row["availability"] not in {"in_stock", "out_of_stock", "unknown"}
                    or row["price_kind"] not in {"cash", "pix", "installment_total", "unknown"}):
                continue
            expiry = row["valid_until"]
            fresh = now - row["collected_at"] <= FRESH_SECONDS
            if expiry is not None:
                if type(expiry) is not int or expiry < row["collected_at"]:
                    continue
                fresh = fresh and expiry > now
            offers.append({"id": row["id"], "product_id": row["product_id"],
                "brand": row["brand"][:64], "model": row["model"][:80],
                "storage_gb": row["storage_gb"], "ram_gb": row["ram_gb"],
                "source_id": row["source_id"], "source_name": source["name"][:64],
                "price_cents": row["price_cents"], "price_kind": row["price_kind"],
                "currency": "BRL", "availability": row["availability"], "url": row["url"],
                "collected_at": timestamp(row["collected_at"]), "freshness": "fresh" if fresh else "stale"})
        return offers, truncated
    finally:
        conn.close()


def search(root: Path, database: Path | None, query: str, now: int, limit: int = 5) -> dict:
    if type(now) is not int or not 0 <= now <= 253402214400 or type(limit) is not int or not 1 <= limit <= 5:
        raise Rejected("invalid_bounds")
    stores, products = configuration(root)
    selected = selected_targets(query, products)
    reply = base(query, now, stores)
    if not selected:
        reply.update(code="untracked_product", summary_pt="O modelo exato não está na lista do BitLupa. POCO X7 e POCO X7 Pro são modelos diferentes; não substituirei uma variante por outra.")
        return reply
    if database is None or not database.exists():
        reply.update(code="bitlupa_database_missing", summary_pt="O BitLupa disponível ainda é uma base de configuração e esquema: não há banco de ofertas coletadas nem coletor executável configurado. Nenhum preço foi inventado. Conclua a primeira fonte verificada do BitLupa antes da comparação.")
        return reply
    if not any(active(source) for source in stores.values()):
        reply.update(code="bitlupa_sources_unavailable", summary_pt="As fontes do BitLupa ainda estão bloqueadas, desativadas ou sem revisão concluída. Nenhuma coleta foi iniciada e nenhum preço foi presumido.")
        return reply
    try:
        offers, truncated = read_offers(database, stores, selected, now)
    except (sqlite3.Error, OSError, Rejected, TypeError, ValueError, KeyError):
        reply.update(code="bitlupa_database_unavailable", summary_pt="O banco local do BitLupa não pôde ser consultado no contrato e prazo esperados. Nenhum preço foi presumido e nenhum arquivo foi alterado.")
        return reply
    offers.sort(key=lambda o: (o["freshness"] != "fresh", o["availability"] != "in_stock", o["price_cents"], o["id"]))
    shown = offers[:limit]
    eligible = [o for o in shown if o["freshness"] == "fresh" and o["availability"] == "in_stock" and o["price_kind"] in {"cash", "pix"}]
    lowest = min(eligible, key=lambda o: (o["price_cents"], o["id"]), default=None)
    reply.update(status="partial" if truncated or not lowest else "completed",
        code="observed_offers" if offers else "no_verified_offers", offers=shown, coverage_limited=truncated,
        lowest_observed_offer_id=lowest["id"] if lowest else None,
        summary_pt=("Ofertas observadas no banco do BitLupa; confira variante, pagamento e horário. O menor preço exibido não inclui frete nem comprova melhor custo-benefício total." if shown else "Nenhuma oferta verificada foi encontrada para o modelo exato nas fontes autorizadas. Fontes bloqueadas/desativadas não foram consultadas e nenhum preço foi inventado."))
    return reply


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--project-root", required=True, type=Path)
    parser.add_argument("--database", type=Path)
    commands = parser.add_subparsers(dest="command", required=True)
    commands.add_parser("capabilities")
    find = commands.add_parser("search")
    find.add_argument("--query", required=True)
    find.add_argument("--max-results", type=int, default=5)
    find.add_argument("--now", type=int)
    args = parser.parse_args(argv)
    try:
        if args.command == "capabilities":
            stores, products = configuration(args.project_root)
            result = {"schema": 1, "service": "bitlupa", "operations": ["stored_offers.search"], "live_collection": False,
                "category": "cellphones", "condition_scope": ["new"], "tracked_products": len(products),
                "tracked_models": sorted({p["model"] for p in products}),
                "sources": [{"id": s["id"], "name": s["name"], "status": s.get("status", "unknown"), "eligible": active(s)} for s in stores.values() if s["id"] in RETAILERS],
                "eligible_sources": sum(active(s) for s in stores.values()),
                "database_configured": args.database is not None and args.database.is_file(),
                "license": "MIT (BitLupa)", "readonly": True}
        else:
            result = search(args.project_root, args.database, args.query, int(time.time()) if args.now is None else args.now, args.max_results)
    except (OSError, ValueError, KeyError, TypeError, json.JSONDecodeError) as error:
        result = {"schema": 1, "status": "blocked", "code": error.code if isinstance(error, Rejected) else "bitlupa_configuration_unavailable",
                  "summary_pt": "A configuração local do BitLupa não pôde ser validada. Nenhum preço foi inventado e nenhum serviço externo foi chamado.", "offers": []}
    encoded = json.dumps(result, ensure_ascii=False, separators=(",", ":"))
    if len(encoded.encode()) > MAX_RESPONSE_BYTES:
        encoded = json.dumps({"schema": 1, "status": "blocked", "code": "bounded_response_exceeded", "offers": []})
    print(encoded)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
