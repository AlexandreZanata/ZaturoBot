#!/usr/bin/env python3
"""Bounded Postiz Public API adapter; exact local drafts precede separately approved writes.

Wire references checked 2026-11-14 (no upstream source code is copied):
https://docs.postiz.com/public-api/introduction
https://docs.postiz.com/public-api/integrations/list
https://docs.postiz.com/public-api/integrations/is-connected
https://docs.postiz.com/public-api/posts/create
https://docs.postiz.com/public-api/posts/list

Postiz is an optional independently hosted AGPL-3.0 service, not a dependency to run on
ZaturoBot's lightweight VPS. API acceptance is not evidence of eventual publication.
"""
from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import fcntl
import json
import math
import os
from pathlib import Path
import re
import stat
import sys
import time
import tempfile
import tomllib
import urllib.error
import urllib.parse
import urllib.request
import uuid

SCHEMA = 1
MAX_INPUT = 16384
MAX_OUTPUT = 16384
MAX_RESPONSE = 128 * 1024
MAX_JOURNAL = 8 * 1024 * 1024
MAX_OPERATIONS = 500
MAX_CONTENT = 1500
MAX_DRAFT_AGE = 1800
PLATFORMS = frozenset({"threads", "mastodon", "bluesky", "telegram", "nostr", "vk"})
IDENTIFIER = re.compile(r"[A-Za-z0-9][A-Za-z0-9_-]{0,127}\Z")
HASH = re.compile(r"[a-f0-9]{64}\Z")
MESSAGES = {
    "social_config_missing": "O serviço de redes sociais não está configurado neste computador. Configure uma instância Postiz separada e uma chave local privada; o YoutubeBot existente permanece disponível pelo próprio serviço.",
    "social_disabled": "O conector de redes sociais está desativado na configuração local.",
    "invalid_config": "A configuração privada do Postiz foi recusada. Confira origem permitida, caminhos, contas e permissões sem exibir credenciais.",
    "invalid_request": "A solicitação não corresponde ao contrato limitado de redes sociais.",
    "unsupported_action": "Esta ação de rede social ainda não tem um adaptador público verificado. Comentários, respostas, mensagens privadas, exclusão e login continuam indisponíveis neste conector.",
    "unsupported_platform": "Esta plataforma exige um adaptador de configurações próprio e ainda não está habilitada. Para YouTube, use o serviço YoutubeBot existente.",
    "integration_not_allowed": "A conta de destino não foi autorizada no cadastro local do Postiz.",
    "media_not_allowed": "A mídia não está no cadastro local de arquivos já enviados ao Postiz. Não aceito URLs ou caminhos arbitrários para upload.",
    "api_authentication": "O Postiz recusou a chave de API. Entre no PC de trabalho e confira a chave e a instância privada; isso não comprova logout de uma rede social específica.",
    "api_permission": "O Postiz recusou a permissão desta operação. Confira a conta, a organização e as permissões no painel do Postiz.",
    "api_unavailable": "O Postiz não respondeu de forma válida dentro dos limites. Verifique a instância e a conexão no computador de trabalho.",
    "api_rate_limited": "O Postiz limitou as solicitações. Nenhuma publicação será repetida automaticamente.",
    "api_not_supported": "A instância Postiz não oferece a rota pública esperada. Confira a versão e o endereço da API; nenhuma rota interna será usada como alternativa.",
    "api_rejected": "O Postiz recusou a operação. Confira o painel e as regras da plataforma; a resposta bruta não é enviada ao chat.",
    "integration_missing": "A conta autorizada não apareceu no Postiz. Conecte a conta correta no painel do PC de trabalho; nenhuma outra conta será escolhida.",
    "integration_disabled": "A conta está desativada no Postiz. Entre no PC de trabalho e confira a conexão da conta antes de publicar.",
    "integration_unknown": "A conta existe, mas o Postiz não forneceu evidência suficiente para confirmar sua disponibilidade. Confira o painel; não presumirei que ela está logada.",
    "owner_approval_required": "A publicação exige a confirmação do dono para o hash exato deste rascunho. Preparar ou ler um texto não autoriza publicação.",
    "local_write_grant_required": "A execução de publicações não foi autorizada na configuração local. O rascunho permanece sem envio.",
    "draft_not_found": "O rascunho local desta operação não foi encontrado. Prepare e revise o conteúdo exato antes de confirmar.",
    "draft_conflict": "O identificador da operação já está ligado a outro conteúdo. Nenhum rascunho foi sobrescrito.",
    "draft_expired": "A aprovação do rascunho expirou. A operação não foi enviada; prepare um novo rascunho para revisão.",
    "config_changed": "A configuração ou a credencial mudou depois da preparação. A aprovação anterior não autoriza a nova configuração.",
    "reconciliation_required": "O resultado desta publicação é incerto. Não repetirei o envio. Confira o painel do Postiz e reconcilie a operação antes de publicar novamente nesta conta.",
    "integration_reconciliation_required": "Há outra publicação incerta ou em andamento nesta conta. Confira essa operação no Postiz antes de um novo envio.",
    "local_state_unavailable": "O histórico privado de operações não pôde ser aberto ou salvo com segurança. Não posso confirmar uma nova publicação; confira operações em andamento antes de outra tentativa.",
    "local_capacity": "O histórico local atingiu seu limite. Revise e arquive as operações com segurança; nenhum registro será removido automaticamente.",
    "clock_regressed": "O relógio local recuou. A execução foi bloqueada para não estender uma aprovação expirada.",
    "post_not_observed": "A API aceitou o pedido, mas o post ainda não apareceu na consulta limitada. Isso não confirma publicação; nenhum envio será repetido.",
}


class SocialError(ValueError):
    def __init__(self, code: str):
        self.code = code if code in MESSAGES else "invalid_request"
        super().__init__(self.code)


class ApiError(SocialError):
    pass


def require(condition: bool, code: str = "invalid_request") -> None:
    if not condition:
        raise SocialError(code)


def canonical(value) -> str:
    return json.dumps(value, sort_keys=True, ensure_ascii=False, separators=(",", ":"), allow_nan=False)


def digest(value) -> str:
    return hashlib.sha256(canonical(value).encode()).hexdigest()


def json_object(data: str | bytes):
    def unique(pairs):
        result = {}
        for key, value in pairs:
            require(key not in result)
            result[key] = value
        return result
    return json.loads(data, object_pairs_hook=unique, parse_constant=lambda _: (_ for _ in ()).throw(SocialError("invalid_request")))


def fields(value, required: set[str], optional: set[str] = frozenset()) -> None:
    require(isinstance(value, dict) and required <= value.keys() and value.keys() <= required | optional)


def identifier(value) -> bool:
    return isinstance(value, str) and IDENTIFIER.fullmatch(value) is not None


def operation(value) -> str:
    require(isinstance(value, str))
    try:
        require(str(uuid.UUID(value)) == value)
    except (ValueError, AttributeError):
        raise SocialError("invalid_request") from None
    return value


def text(value, maximum: int) -> bool:
    return (isinstance(value, str) and bool(value.strip()) and len(value.encode()) <= maximum
            and not any(ord(c) < 32 and c not in "\n\t" for c in value)
            and not any(marker in value for marker in ("oc_sk_", "-----BEGIN ", "AGE-SECRET-", "POSTIZ_API_KEY=", "ZATURO_OWNER_TOKEN=")))


def private_path(path: Path, exists: bool = True, maximum: int = MAX_INPUT) -> Path:
    require(path.is_absolute(), "invalid_config")
    require(not any(p.is_symlink() for p in [path, *path.parents]), "invalid_config")
    if exists:
        fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
        with os.fdopen(fd, "rb") as stream:
            info = os.fstat(stream.fileno())
            require(stat.S_ISREG(info.st_mode) and info.st_uid == os.getuid()
                    and not info.st_mode & 0o077 and info.st_size <= maximum, "invalid_config")
    return path


def private_bytes(path: Path, maximum: int = MAX_INPUT) -> bytes:
    private_path(path, maximum=maximum)
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    with os.fdopen(fd, "rb") as stream:
        data = stream.read(maximum + 1)
    require(len(data) <= maximum, "invalid_config")
    return data


def origin(value: str) -> str:
    require(isinstance(value, str) and len(value) <= 512 and not any(ord(c) < 33 for c in value), "invalid_config")
    try:
        parsed = urllib.parse.urlsplit(value)
        require(parsed.scheme in {"https", "http"} and bool(parsed.hostname) and not parsed.username
                and not parsed.password and parsed.path in {"", "/"} and not parsed.query and not parsed.fragment,
                "invalid_config")
        require(parsed.port is None or 1 <= parsed.port <= 65535, "invalid_config")
        require(parsed.scheme == "https" or parsed.hostname in {"127.0.0.1", "::1"}, "invalid_config")
        return urllib.parse.urlunsplit((parsed.scheme, parsed.netloc, "", "", ""))
    except ValueError:
        raise SocialError("invalid_config") from None


class Config:
    def __init__(self, path: Path):
        if not path.exists():
            raise SocialError("social_config_missing")
        self.path = path
        value = tomllib.loads(private_bytes(path).decode())
        require(type(value.get("schema")) is int and value.get("schema") == 1 and type(value.get("enabled")) is bool, "invalid_config")
        if not value["enabled"]:
            raise SocialError("social_disabled")
        required = {"schema", "enabled", "api_base", "allowed_origin", "api_key_file", "journal_file", "allowed_integrations"}
        require(required <= value.keys() and value.keys() <= required | {"allow_execute", "allowed_media"}, "invalid_config")
        self.allowed_origin = origin(value["allowed_origin"])
        self.api_base = value["api_base"].rstrip("/") if isinstance(value["api_base"], str) else ""
        parsed = urllib.parse.urlsplit(self.api_base)
        require(parsed.path in {"/public/v1", "/api/public/v1"} and not parsed.query and not parsed.fragment, "invalid_config")
        require(origin(urllib.parse.urlunsplit((parsed.scheme, parsed.netloc, "", "", ""))) == self.allowed_origin, "invalid_config")
        self.allow_execute = value.get("allow_execute", False)
        require(type(self.allow_execute) is bool, "invalid_config")
        integrations = value["allowed_integrations"]
        require(isinstance(integrations, dict) and 1 <= len(integrations) <= 8
                and all(identifier(key) and isinstance(platform, str) and identifier(platform) for key, platform in integrations.items()), "invalid_config")
        self.integrations = integrations
        media = value.get("allowed_media", {})
        require(isinstance(media, dict) and len(media) <= 32, "invalid_config")
        for key, url in media.items():
            require(identifier(key) and isinstance(url, str) and len(url.encode()) <= 512, "invalid_config")
            asset = urllib.parse.urlsplit(url)
            require(asset.path.startswith("/") and not asset.query and not asset.fragment and not any(ord(c) < 33 for c in url), "invalid_config")
            require(origin(urllib.parse.urlunsplit((asset.scheme, asset.netloc, "", "", ""))) == self.allowed_origin, "invalid_config")
        self.media = media
        self.journal_file = private_path(Path(value["journal_file"]), exists=False)
        key_file = Path(value["api_key_file"])
        key = private_bytes(key_file, 8192).decode().strip()
        if key.startswith("POSTIZ_API_KEY="):
            key = key.removeprefix("POSTIZ_API_KEY=")
        require(32 <= len(key) <= 4096 and all(33 <= ord(c) <= 126 for c in key), "invalid_config")
        require(key not in self.api_base and all(key not in identity for identity in integrations)
                and key not in canonical(media), "invalid_config")
        self.key = key
        # This fingerprint is private journal state, never output or sent to Postiz.
        self.binding = digest({"api_base": self.api_base, "key_sha256": hashlib.sha256(key.encode()).hexdigest(),
                               "integrations": integrations, "media": media, "allow_execute": self.allow_execute})


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, request, fp, code, message, headers, newurl):
        return None


class Api:
    def __init__(self, config: Config):
        self.config = config
        self.opener = urllib.request.build_opener(urllib.request.ProxyHandler({}), NoRedirect())

    def request(self, path: str, body=None):
        require(path.startswith("/") and not path.startswith("//"))
        data = None if body is None else canonical(body).encode()
        require(data is None or len(data) <= MAX_INPUT)
        request = urllib.request.Request(self.config.api_base + path, data=data,
            headers={"Authorization": self.config.key, "Content-Type": "application/json", "Cache-Control": "no-store"})
        try:
            deadline = time.monotonic() + 6
            with self.opener.open(request, timeout=2) as response:
                output = bytearray()
                while True:
                    if time.monotonic() >= deadline:
                        raise ApiError("api_unavailable")
                    chunk = response.read1(8192)
                    if not chunk:
                        break
                    output.extend(chunk)
                    if len(output) > MAX_RESPONSE:
                        raise ApiError("api_unavailable")
                try:
                    return json_object(output)
                except (SocialError, ValueError, UnicodeError, RecursionError):
                    raise ApiError("api_unavailable") from None
        except urllib.error.HTTPError as error:
            status = error.code
            error.close()
            raise ApiError({401: "api_authentication", 403: "api_permission", 404: "api_not_supported", 429: "api_rate_limited"}.get(status, "api_rejected")) from None
        except (OSError, urllib.error.URLError, ValueError, UnicodeError) as error:
            if isinstance(error, SocialError):
                raise
            raise ApiError("api_unavailable") from None


def moment(now: int) -> str:
    return dt.datetime.fromtimestamp(now, dt.timezone.utc).isoformat(timespec="seconds").replace("+00:00", "Z")


def parse_moment(value) -> int:
    require(isinstance(value, str) and re.fullmatch(r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z", value) is not None)
    try:
        return int(dt.datetime.strptime(value, "%Y-%m-%dT%H:%M:%SZ").replace(tzinfo=dt.timezone.utc).timestamp())
    except ValueError:
        raise SocialError("invalid_request") from None


def blocked(code: str, identity: str | None = None) -> dict:
    result = {"schema": SCHEMA, "status": "blocked", "code": code, "summary_pt": MESSAGES[code]}
    if identity is not None:
        result["operation_id"] = identity
    return result


def capabilities() -> dict:
    return {"schema": SCHEMA, "status": "completed", "provider": "postiz-public-v1",
            "operations": ["readiness", "prepare", "execute", "status"], "actions": ["publish", "schedule", "draft"],
            "platforms": sorted(PLATFORMS), "max_content_bytes": MAX_CONTENT, "max_media_ids": 4,
            "unsupported": ["comment_reply", "comment_list", "direct_message", "delete", "account_login", "media_upload", "youtube"],
            "summary_pt": "Base Postiz para consultar cadastros, preparar rascunho exato, enviar após confirmação e consultar o resultado. Requer instância e contas configuradas; comentários, mensagens e plataformas sem adaptador permanecem indisponíveis."}


class Journal:
    """Private provider records, serialized under flock and published with fsync+rename.

    The coordinator's SQL belongs to zaturo-db. This independent provider transport
    stores only its own bounded draft/intent receipts, never account credentials.
    """
    def __init__(self, config: Config):
        self.path = config.journal_file
        parent = self.path.parent
        private_path(parent, exists=False)
        if not parent.exists():
            parent.mkdir(mode=0o700)
        info = parent.stat()
        require(stat.S_ISDIR(info.st_mode) and info.st_uid == os.getuid() and not info.st_mode & 0o077, "invalid_config")
        lock = self.path.with_name(self.path.name + ".lock")
        private_path(lock, exists=False)
        self.lock = os.open(lock, os.O_RDWR | os.O_CREAT | os.O_NOFOLLOW | os.O_NONBLOCK, 0o600)
        info = os.fstat(self.lock)
        if not (stat.S_ISREG(info.st_mode) and info.st_uid == os.getuid() and not info.st_mode & 0o077 and info.st_size == 0):
            os.close(self.lock)
            raise SocialError("local_state_unavailable")
        self.in_transaction = False
        self.dirty = False
        self.state = None

    def begin(self):
        require(not self.in_transaction, "local_state_unavailable")
        try:
            fcntl.flock(self.lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            deadline = time.monotonic() + 1
            while True:
                if time.monotonic() >= deadline:
                    raise SocialError("local_state_unavailable")
                time.sleep(.02)
                try:
                    fcntl.flock(self.lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
                    break
                except BlockingIOError:
                    continue
        self.in_transaction = True
        self.dirty = False
        try:
            if self.path.exists():
                state = json_object(private_bytes(self.path, MAX_JOURNAL))
                self.validate(state)
                self.state = state
            else:
                private_path(self.path, exists=False)
                self.state = {"origin": "zaturobot.social.v1", "schema": 1, "last_seen": None, "operations": {}}
        except (ValueError, TypeError, KeyError, UnicodeError, RecursionError, OSError):
            self.rollback()
            raise SocialError("local_state_unavailable") from None

    @staticmethod
    def validate(state):
        fields(state, {"origin", "schema", "last_seen", "operations"})
        require(state["origin"] == "zaturobot.social.v1" and type(state["schema"]) is int and state["schema"] == 1)
        require(state["last_seen"] is None or type(state["last_seen"]) is int and state["last_seen"] >= 0)
        require(isinstance(state["operations"], dict) and len(state["operations"]) <= MAX_OPERATIONS)
        required = {"id", "request_hash", "config_hash", "draft_hash", "request_json", "body_json", "state",
                    "created_at", "expires_at", "reply_json", "post_id", "integration_id"}
        for identity, row in state["operations"].items():
            require(operation(identity) == identity)
            fields(row, required)
            require(row["id"] == identity and identifier(row["integration_id"]))
            require(all(isinstance(row[key], str) and HASH.fullmatch(row[key]) for key in ("request_hash", "config_hash", "draft_hash")))
            require(row["state"] in {"prepared", "intent", "accepted", "uncertain"})
            require(type(row["created_at"]) is int and row["created_at"] >= 0 and type(row["expires_at"]) is int
                    and row["expires_at"] == row["created_at"] + MAX_DRAFT_AGE)
            require(all(isinstance(row[key], str) and len(row[key].encode()) <= MAX_INPUT for key in ("request_json", "body_json")))
            request, body = json_object(row["request_json"]), json_object(row["body_json"])
            require(isinstance(request, dict) and isinstance(body, dict)
                    and request.get("operation_id") == identity and request.get("integration_id") == row["integration_id"])
            require(digest(request) == row["request_hash"]
                    and digest({"operation_id": identity, "binding": row["config_hash"], "payload": body}) == row["draft_hash"])
            require(row["reply_json"] is None or isinstance(row["reply_json"], str) and len(row["reply_json"].encode()) <= MAX_OUTPUT)
            require(row["post_id"] is None or identifier(row["post_id"]))
            if row["state"] == "accepted":
                require(row["post_id"] is not None and row["reply_json"] is not None)
                reply = json_object(row["reply_json"])
                require(isinstance(reply, dict) and reply.get("operation_id") == identity and reply.get("status") == "accepted" and reply.get("post_id") == row["post_id"])

    def commit(self):
        require(self.in_transaction, "local_state_unavailable")
        if self.dirty:
            self.validate(self.state)
            data = canonical(self.state).encode()
            require(len(data) <= MAX_JOURNAL, "local_capacity")
            descriptor, temporary = tempfile.mkstemp(prefix=".social-receipt-", dir=self.path.parent)
            try:
                with os.fdopen(descriptor, "wb") as stream:
                    os.fchmod(stream.fileno(), 0o600)
                    stream.write(data)
                    stream.flush()
                    os.fsync(stream.fileno())
                if self.path.exists():
                    private_path(self.path, maximum=MAX_JOURNAL)
                else:
                    private_path(self.path, exists=False)
                os.replace(temporary, self.path)
                directory = os.open(self.path.parent, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
                try:
                    os.fsync(directory)
                finally:
                    os.close(directory)
            finally:
                if os.path.exists(temporary):
                    os.unlink(temporary)
        self.rollback()

    def rollback(self):
        if self.in_transaction:
            self.in_transaction = False
            self.dirty = False
            self.state = None
            fcntl.flock(self.lock, fcntl.LOCK_UN)

    def close(self):
        self.rollback()
        os.close(self.lock)

    def get(self, identity):
        if self.in_transaction:
            return self.state["operations"].get(identity)
        self.begin()
        try:
            row = self.state["operations"].get(identity)
            return None if row is None else dict(row)
        finally:
            self.rollback()

    def count(self):
        require(self.in_transaction, "local_state_unavailable")
        return len(self.state["operations"])

    def add(self, row):
        require(self.in_transaction and row["id"] not in self.state["operations"], "local_state_unavailable")
        self.state["operations"][row["id"]] = row
        self.dirty = True

    def clock(self, now):
        require(self.in_transaction, "local_state_unavailable")
        previous = self.state["last_seen"]
        require(previous is None or now >= previous, "clock_regressed")
        self.state["last_seen"] = now
        self.dirty = True

    def inflight(self, integration, excluded):
        require(self.in_transaction, "local_state_unavailable")
        return next((row["id"] for row in self.state["operations"].values()
            if row["id"] != excluded and row["integration_id"] == integration and row["state"] in {"intent", "uncertain"}), None)

    def mark_intent(self, identity):
        row = self.get(identity)
        require(self.in_transaction and row is not None and row["state"] == "prepared", "reconciliation_required")
        row["state"] = "intent"
        self.dirty = True

    def finish(self, identity, state, reply, post_id=None):
        row = self.get(identity)
        require(self.in_transaction and row is not None, "local_state_unavailable")
        if row["state"] == "intent":
            row.update(state=state, reply_json=canonical(reply), post_id=post_id)
            self.dirty = True


def normalize_prepare(request: dict) -> dict:
    fields(request, {"operation_id", "action", "integration_id", "content"}, {"scheduled_at", "media_ids"})
    identity = operation(request["operation_id"])
    require(isinstance(request["action"], str) and request["action"] in {"publish", "schedule", "draft"}, "unsupported_action")
    require(identifier(request["integration_id"]))
    require(text(request["content"], MAX_CONTENT))
    media = request.get("media_ids", [])
    require(isinstance(media, list) and len(media) <= 4 and all(identifier(value) for value in media)
            and len(set(media)) == len(media), "media_not_allowed")
    if request["action"] == "schedule":
        parse_moment(request.get("scheduled_at"))
    else:
        require("scheduled_at" not in request)
    return {"operation_id": identity, "action": request["action"], "integration_id": request["integration_id"],
            "content": request["content"], "media_ids": media, "scheduled_at": request.get("scheduled_at")}


def draft_reply(row) -> dict:
    request = json_object(row["request_json"])
    payload = json_object(row["body_json"])
    kind = {"publish": "Publicação imediata", "schedule": "Agendamento", "draft": "Rascunho no Postiz"}[request["action"]]
    platform = payload["posts"][0]["settings"]["__type"]
    preview = f"{kind}\nConta: {request['integration_id']} · {platform}\n"
    if request["action"] == "schedule":
        preview += f"Data UTC: {request['scheduled_at']}\n"
    preview += "\n" + request["content"]
    if request["media_ids"]:
        preview += "\n\nMídias já cadastradas: " + ", ".join(request["media_ids"])
    return {"schema": SCHEMA, "status": "prepared", "code": "owner_review_required", "operation_id": row["id"],
            "draft_hash": row["draft_hash"], "expires_at": moment(row["expires_at"]), "payload": payload,
            "preview_pt": preview, "summary_pt": "Rascunho local preparado para revisão. Nenhuma solicitação de publicação foi enviada ao Postiz.",
            "provider": "postiz-public-v1", "approval_required": True}


def prepare(config: Config, request: dict, now: int) -> dict:
    request = normalize_prepare(request)
    identity = request["operation_id"]
    require(config.key not in request["content"])
    platform = config.integrations.get(request["integration_id"])
    require(platform is not None, "integration_not_allowed")
    require(platform in PLATFORMS, "unsupported_platform")
    require(all(item in config.media for item in request["media_ids"]), "media_not_allowed")
    journal = Journal(config)
    try:
        journal.begin()
        journal.clock(now)
        old = journal.get(identity)
        if old:
            require(old["request_hash"] == digest(request), "draft_conflict")
            require(old["config_hash"] == config.binding, "config_changed")
            if old["state"] == "accepted":
                journal.commit()
                return json_object(old["reply_json"])
            if old["state"] in {"intent", "uncertain"}:
                journal.commit()
                return blocked("reconciliation_required", identity)
            require(now <= old["expires_at"], "draft_expired")
            journal.commit()
            return draft_reply(old)
        require(journal.count() < MAX_OPERATIONS, "local_capacity")
        date = request["scheduled_at"] or moment(now)
        if request["action"] == "schedule":
            require(now + 60 <= parse_moment(date) <= now + 365 * 86400)
        body = {"type": {"publish": "now", "schedule": "schedule", "draft": "draft"}[request["action"]],
                "date": date, "shortLink": False, "tags": [],
                "posts": [{"integration": {"id": request["integration_id"]},
                           "value": [{"content": request["content"], "image": [{"id": key, "path": config.media[key]} for key in request["media_ids"]]}],
                           "settings": {"__type": platform}}]}
        approval_hash = digest({"operation_id": identity, "binding": config.binding, "payload": body})
        journal.add({"id": identity, "request_hash": digest(request), "config_hash": config.binding,
                     "draft_hash": approval_hash, "request_json": canonical(request), "body_json": canonical(body),
                     "state": "prepared", "created_at": now, "expires_at": now + MAX_DRAFT_AGE,
                     "reply_json": None, "post_id": None, "integration_id": request["integration_id"]})
        row = journal.get(identity)
        reply = draft_reply(row)
        require(len(canonical(reply).encode()) <= MAX_OUTPUT)
        journal.commit()
        return reply
    finally:
        if journal.in_transaction:
            journal.rollback()
        journal.close()


def readiness(config: Config, request: dict, api: Api | None = None) -> dict:
    fields(request, set(), {"integration_id"})
    if "integration_id" in request:
        require(request["integration_id"] in config.integrations, "integration_not_allowed")
    api = api or Api(config)
    connected = api.request("/is-connected")
    require(isinstance(connected, dict) and type(connected.get("connected")) is bool, "api_unavailable")
    require(connected["connected"], "api_authentication")
    records = api.request("/integrations")
    require(isinstance(records, list) and len(records) <= 200, "api_unavailable")
    result = []
    for identity, platform in config.integrations.items():
        if request.get("integration_id", identity) != identity:
            continue
        matches = [record for record in records if isinstance(record, dict) and record.get("id") == identity]
        record = matches[0] if len(matches) == 1 else None
        if record is None:
            state = "missing"
        elif record.get("identifier") != platform or type(record.get("disabled")) is not bool:
            state = "unknown"
        else:
            state = "disabled" if record["disabled"] else "registered"
        result.append({"id": identity, "platform": platform, "state": state,
                       "name": record["name"][:80] if record and text(record.get("name"), 320) and config.key not in record["name"] else identity,
                       "supported": platform in PLATFORMS})
    return {"schema": SCHEMA, "status": "completed", "code": "registry_observed", "accounts": result,
            "summary_pt": "A chave do Postiz foi validada e o cadastro das contas foi consultado. Conta cadastrada não comprova login ativo na rede; confira autenticações e desafios no painel do PC de trabalho."}


def execute(config: Config, request: dict, now: int, api: Api | None = None) -> dict:
    fields(request, {"operation_id", "approved_draft_hash", "owner_approved"})
    identity = operation(request["operation_id"])
    require(request["owner_approved"] is True and isinstance(request["approved_draft_hash"], str)
            and HASH.fullmatch(request["approved_draft_hash"]) is not None, "owner_approval_required")
    started = time.monotonic()
    journal = Journal(config)
    try:
        journal.begin()
        row = journal.get(identity)
        require(row is not None, "draft_not_found")
        require(row["draft_hash"] == request["approved_draft_hash"], "owner_approval_required")
        require(row["config_hash"] == config.binding, "config_changed")
        if row["state"] == "accepted":
            journal.commit()
            return json_object(row["reply_json"])
        require(row["state"] == "prepared", "reconciliation_required")
        journal.clock(now)
        require(row["created_at"] <= now <= row["expires_at"], "draft_expired")
        require(config.allow_execute, "local_write_grant_required")
        body = json_object(row["body_json"])
        if body["type"] == "schedule":
            require(parse_moment(body["date"]) >= now + 10, "draft_expired")
        pending = journal.inflight(row["integration_id"], identity)
        require(pending is None, "integration_reconciliation_required")
        # Read-only preflight precedes the durable write intent. A failed preflight leaves
        # the exact draft available without claiming that a publication happened.
        journal.commit()
        api = api or Api(config)
        observed = readiness(config, {"integration_id": row["integration_id"]}, api)
        account = observed["accounts"][0]
        require(account["state"] == "registered", {"missing": "integration_missing", "disabled": "integration_disabled"}.get(account["state"], "integration_unknown"))
        # Recheck local revocation/config and elapsed approval time immediately before
        # durable intent, including time spent on the read-only network preflight.
        require(Config(config.path).binding == config.binding, "config_changed")
        executing_now = now + math.ceil(time.monotonic() - started)
        journal.begin()
        journal.clock(executing_now)
        current = journal.get(identity)
        require(current["state"] == "prepared", "reconciliation_required")
        require(current["created_at"] <= executing_now <= current["expires_at"], "draft_expired")
        if body["type"] == "schedule":
            require(parse_moment(body["date"]) >= executing_now + 10, "draft_expired")
        pending = journal.inflight(row["integration_id"], identity)
        require(pending is None, "integration_reconciliation_required")
        journal.mark_intent(identity)
        journal.commit()
        # From this commit onward every unknown/error outcome is fenced. No retry or
        # guessed content match can prove that a remote publication did not occur.
        try:
            result = api.request("/posts", body)
            require(isinstance(result, list) and len(result) == 1 and isinstance(result[0], dict)
                    and identifier(result[0].get("postId")) and config.key not in result[0]["postId"] and result[0].get("integration") == row["integration_id"], "api_unavailable")
            reply = {"schema": SCHEMA, "status": "accepted", "code": "postiz_accepted", "operation_id": identity,
                     "draft_hash": row["draft_hash"], "post_id": result[0]["postId"], "integration_id": row["integration_id"],
                     "summary_pt": "O Postiz aceitou a solicitação e retornou o identificador do post. Isso ainda não confirma publicação na rede; o resultado deve ser observado pela consulta de status."}
            journal.begin()
            journal.finish(identity, "accepted", reply, result[0]["postId"])
            journal.commit()
            return reply
        except (SocialError, OSError, ValueError, TypeError, KeyError) as error:
            if journal.in_transaction:
                journal.rollback()
            reply = blocked("reconciliation_required", identity)
            if isinstance(error, ApiError):
                reply["provider_code"] = error.code
                reply["summary_pt"] = MESSAGES[error.code] + " " + MESSAGES["reconciliation_required"]
            try:
                journal.begin()
                journal.finish(identity, "uncertain", reply)
                journal.commit()
            except (OSError, SocialError):
                # The already committed intent remains the recovery fence if saving
                # the final response fails. Never imply that no remote effect occurred.
                if journal.in_transaction:
                    journal.rollback()
            return reply
    finally:
        if journal.in_transaction:
            journal.rollback()
        journal.close()


def status(config: Config, request: dict, api: Api | None = None) -> dict:
    fields(request, {"operation_id"})
    identity = operation(request["operation_id"])
    journal = Journal(config)
    try:
        row = journal.get(identity)
        require(row is not None, "draft_not_found")
        require(row["config_hash"] == config.binding, "config_changed")
        if row["state"] in {"intent", "uncertain"}:
            return blocked("reconciliation_required", identity)
        if row["state"] == "prepared":
            return draft_reply(row)
        body = json_object(row["body_json"])
        date = parse_moment(body["date"])
        query = urllib.parse.urlencode({"startDate": moment(max(0, date - 86400)), "endDate": moment(date + 86400)})
        result = (api or Api(config)).request("/posts?" + query)
        require(isinstance(result, dict) and isinstance(result.get("posts"), list) and len(result["posts"]) <= 500, "api_unavailable")
        found = [post for post in result["posts"] if isinstance(post, dict) and post.get("id") == row["post_id"]
                 and isinstance(post.get("integration"), dict) and post["integration"].get("id") == row["integration_id"]]
        if len(found) != 1:
            return blocked("post_not_observed", identity)
        state = found[0].get("state")
        status_name, message = {
            "PUBLISHED": ("published", "O Postiz registrou este post como publicado. Esta é a evidência de status da API; nenhuma captura de tela foi produzida por este conector."),
            "QUEUE": ("scheduled", "O Postiz mantém este post na fila. O agendamento não equivale a publicação concluída."),
            "DRAFT": ("drafted", "O Postiz registrou o rascunho. Ele ainda não foi publicado nem agendado."),
            "ERROR": ("failed", "O Postiz marcou falha nesta publicação. Confira a conta e o erro no painel; nenhum envio será repetido automaticamente."),
        }.get(state, ("unknown", "O Postiz retornou um estado de publicação ainda não reconhecido. Verifique o painel; não afirmarei que a ação terminou."))
        return {"schema": SCHEMA, "status": status_name, "code": "postiz_status_observed", "operation_id": identity,
                "post_id": row["post_id"], "integration_id": row["integration_id"], "summary_pt": message,
                "source": "postiz_public_api", "screenshot_created": False}
    finally:
        journal.close()


def dispatch(command: str, config_path: Path | None, request: dict, now: int | None = None):
    require(isinstance(request, dict))
    if command == "capabilities":
        fields(request, set())
        return capabilities()
    require(config_path is not None, "social_config_missing")
    config = Config(config_path)
    now = int(time.time()) if now is None else now
    require(type(now) is int and 0 <= now <= 253402214400)
    if command == "readiness":
        return readiness(config, request)
    if command == "prepare":
        return prepare(config, request, now)
    if command == "execute":
        return execute(config, request, now)
    if command == "status":
        return status(config, request)
    raise SocialError("unsupported_action")


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=["capabilities", "readiness", "prepare", "execute", "status"])
    parser.add_argument("--config", type=Path)
    args = parser.parse_args(argv)
    os.umask(0o077)
    try:
        data = sys.stdin.buffer.read(MAX_INPUT + 1)
        require(len(data) <= MAX_INPUT)
        request = json_object(data) if data.strip() else {}
        result = dispatch(args.command, args.config, request)
    except SocialError as error:
        result = blocked(error.code)
    except OSError:
        result = blocked("local_state_unavailable")
    except (ValueError, TypeError, KeyError, UnicodeError, OverflowError, RecursionError):
        result = blocked("invalid_request")
    encoded = canonical(result)
    if len(encoded.encode()) > MAX_OUTPUT:
        encoded = canonical(blocked("invalid_request"))
    print(encoded)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
