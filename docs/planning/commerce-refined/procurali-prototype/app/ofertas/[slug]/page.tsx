import Link from "next/link";
import { notFound } from "next/navigation";
import { SITE_META } from "@/data/demo-catalog.mjs";
import { getCatalog, getOffer } from "@/lib/catalog";
import {
  buildOfficialAffiliateAnchorProps,
  formatBrl,
  formatTelegramHtmlOffer,
  formatWhatsappWebAssistedText,
  isOfferFresh,
  verifySevenDaySample,
} from "@/lib/contracts.mjs";
import { breadcrumbLd, pageMetadata } from "@/lib/seo";
import { Badge, Breadcrumbs, JsonLd, Notice } from "@/components/site/primitives";
import { AffiliateCta, MiniOfferCard, PriceBlock, StatusBadge, mediaSrc } from "@/components/site/offer";
import { SaveButton } from "@/components/site/saved";
import { CopyButton } from "@/components/site/copy-button";
import { OfferGallery } from "@/components/pages/offer-gallery";

type Params = { params: Promise<{ slug: string }> };

export const dynamicParams = false;

export function generateStaticParams() {
  return getCatalog().offers.map((o) => ({ slug: o.slug }));
}

export async function generateMetadata({ params }: Params) {
  const offer = getOffer((await params).slug);
  if (!offer) return {};
  const live = !offer.isDemoFixture && offer.state === "approved";
  return pageMetadata({
    route: `/ofertas/${offer.slug}/`,
    title: live ? offer.title : `${offer.title} (${offer.state === "approved" ? "demo" : offer.state})`,
    description: live
      ? `${offer.oneLineReason} Registro real do Mercado Livre observado em ${offer.observedAtDisplay}.`
      : `${offer.oneLineReason} Dados demonstrativos fictícios com horário de observação e limitações.`,
    ogImage: offer.imagePath,
  });
}

function SectionHeading({ children, sub }: { icon?: unknown; children: React.ReactNode; sub?: string }) {
  return (
    <div className="mb-6">
      <h2 className="text-2xl font-semibold text-ink">{children}</h2>
      {sub && <p className="mt-1.5 max-w-2xl text-sm leading-relaxed text-muted">{sub}</p>}
    </div>
  );
}

export default async function OfferDetailPage({ params }: Params) {
  const { slug } = await params;
  const offer = getOffer(slug);
  if (!offer) notFound();
  const catalog = getCatalog();
  const category = catalog.categories.find((c) => c.slug === offer.categorySlug);
  const rights = catalog.mediaRights.find((m) => m.id === offer.mediaRightsRef);
  const evidence = catalog.evidenceSources.filter((e) => (offer.evidenceRefs || []).includes(e.id));
  const freshness = isOfferFresh(offer, SITE_META.prototypeReferenceTimeUtc);
  const sevenDay = verifySevenDaySample(offer);
  const cta = buildOfficialAffiliateAnchorProps(offer, catalog.mediaRights);
  const alternatives = catalog.offers.filter((o) => (offer.alternativeOfferSlugs || []).includes(o.slug));
  const gallery = (offer.galleryPaths || []).map((g) => ({ src: mediaSrc(g.path), alt: g.alt }));
  const specs = offer.specs || [];
  const variants = offer.variants || [];
  const price = formatBrl(offer.priceBrlCents);
  const live = !offer.isDemoFixture && offer.state === "approved";
  const availability = offer.availability === "InStock" ? "Em estoque no anúncio (snapshot)" : offer.availability || "";
  const route = `/ofertas/${offer.slug}/`;

  return (
    <>
      <JsonLd
        data={[
          breadcrumbLd([
            { name: "Início", path: "/" },
            { name: "Ofertas", path: "/ofertas/" },
            { name: offer.title, path: route },
          ]),
        ]}
      />
      <div className="border-b border-line">
        <div className="container-site pt-8 pb-16">
          <Breadcrumbs
            items={[
              { label: "Início", href: "/" },
              { label: "Ofertas", href: "/ofertas/" },
              category ? { label: category.name, href: `/categorias/${category.slug}/` } : { label: "Categoria" },
              { label: offer.title },
            ]}
          />

          <div className="mb-10">
            {offer.state === "stale" && (
              <Notice variant="warning">
                <p>
                  <strong>Preço em atualização (mais de 30 minutos):</strong> este registro foi observado há cerca de{" "}
                  {freshness.ageMinutes} minutos em relação à referência do protótipo ({SITE_META.prototypeReferenceTimeLabelBrt}).
                  Conforme nossa política de frescor, o link comercial permanece pausado até nova checagem.
                </p>
              </Notice>
            )}
            {offer.state === "expired" && (
              <Notice variant="warning">
                <p>
                  <strong>Oferta encerrada:</strong> este item pertence ao arquivo histórico e não deve ser tratado como preço vigente.
                </p>
              </Notice>
            )}
            {offer.state === "missing_rights" && (
              <Notice variant="danger">
                <p>
                  <strong>Bloqueio por direitos de mídia:</strong> os direitos de imagem do item real ainda não foram confirmados (
                  {rights?.licenseType || "pendente"}). Nenhuma ação de compra é liberada.
                </p>
              </Notice>
            )}
            {live && (
              <Notice variant="live">
                <p>
                  <strong>
                    Promoção real em teste ({offer.marketplaceLabel} Afiliados · {offer.productId || offer.id}):
                  </strong>{" "}
                  dados, especificações e variações capturados em {offer.observedAtDisplay} via hub autenticado do proprietário.
                  Preço, frete por CEP e disponibilidade devem ser confirmados na loja oficial ao clicar.
                </p>
              </Notice>
            )}
            {offer.isDemoFixture && offer.state === "approved" && (
              <Notice>
                <p>
                  <strong>Item fictício de demonstração:</strong> catálogo ilustrativo sem vínculo com produto real nem link oficial ativo.
                </p>
              </Notice>
            )}
          </div>

          <div className="grid gap-x-16 gap-y-10 lg:grid-cols-[1.1fr_0.9fr] [&>*]:min-w-0">
            <figure>
              <OfferGallery main={{ src: mediaSrc(offer.imagePath), alt: offer.imageAlt }} images={gallery} />
              <figcaption className="mt-3 text-xs text-muted">{rights?.attributionText || "Ilustração técnica demonstrativa."}</figcaption>
            </figure>

            <div className="lg:sticky lg:top-24 lg:self-start">
              <div className="mb-4 flex flex-wrap items-center gap-x-4 gap-y-1">
                <StatusBadge offer={offer} />
                {offer.cardBadge && <Badge>{offer.cardBadge}</Badge>}
                {offer.sponsored && <Badge tone="warning">Patrocinado no ML</Badge>}
              </div>
              <p className="text-sm text-muted">{offer.merchant}</p>
              <h1 className="mt-2 text-3xl leading-[1.1] font-semibold text-ink sm:text-4xl">{offer.title}</h1>
              <p className="mt-2 text-sm text-neutral-600">
                {offer.brand && offer.model && (
                  <>
                    <strong>Marca/Modelo:</strong> {offer.brand} · {offer.model} ·{" "}
                  </>
                )}
                <strong>Variante exata:</strong> {offer.exactVariant}
              </p>
              <div className="mt-4 flex flex-wrap gap-x-4 gap-y-1">
                {sevenDay.eligible ? (
                  <Badge tone="emerald">{sevenDay.label}</Badge>
                ) : (
                  <Badge>Observação pontual datada (sem série de 7 dias)</Badge>
                )}
                {offer.marketplaceRating && (
                  <Badge tone="neutral">
                    Avaliação ML: {offer.marketplaceRating}
                    {offer.marketplaceSalesLabel ? ` (${offer.marketplaceSalesLabel})` : ""}
                  </Badge>
                )}
                {availability && <Badge>{availability}</Badge>}
              </div>

              <div className="mt-8 border-t border-line pt-8">
                <PriceBlock offer={offer} size="lg" />
                <dl className="mt-5 space-y-2 text-sm">
                  <div>
                    <dt className="inline font-semibold">Condição: </dt>
                    <dd className="inline text-neutral-700">{offer.paymentConditions}</dd>
                  </div>
                  {offer.returnDays ? (
                    <div>
                      <dt className="inline font-semibold">Devolução: </dt>
                      <dd className="inline text-neutral-700">Grátis em até {offer.returnDays} dias informada no anúncio oficial.</dd>
                    </div>
                  ) : null}
                  {offer.couponCondition && (
                    <div>
                      <dt className="inline font-semibold">Regra do cupom: </dt>
                      <dd className="inline text-neutral-700">{offer.couponCondition}</dd>
                    </div>
                  )}
                  <div className="flex gap-1.5 text-muted">
                                        <span>
                      <strong>Frete:</strong> {offer.freightStatus}
                    </span>
                  </div>
                  <div className="flex gap-1.5 text-muted">
                                        <span>
                      <strong>Observado em:</strong> {offer.observedAtDisplay} · <strong>Fonte:</strong> {offer.program}
                    </span>
                  </div>
                </dl>
                {offer.validUntilDisplay && (
                  <p className="mt-3 text-sm text-amber-800">{offer.validUntilDisplay}</p>
                )}
                {(offer.couponCode || offer.shareId) && (
                  <div className="mt-4 flex flex-wrap gap-2">
                    {offer.couponCode && (
                      <CopyButton pill text={offer.couponCode} label={<>Copiar código do cupom: <strong>{offer.couponCode}</strong></>} />
                    )}
                    {offer.shareId && (
                      <CopyButton pill text={offer.shareId} label={<>Copiar código buscador ML: <strong>{offer.shareId}</strong></>} />
                    )}
                  </div>
                )}
                <div className="mt-7 grid gap-2 sm:grid-cols-[1fr_auto]">
                  <AffiliateCta offer={offer} mediaRights={catalog.mediaRights} className="w-full py-3" />
                  <SaveButton slug={offer.slug} title={offer.title} priceLabel={price} className="w-full" />
                </div>
                <div className="mt-6 space-y-1 text-xs leading-relaxed">
                  <p className="text-ink">{SITE_META.affiliateDisclosure}</p>
                  <p className="text-muted">{SITE_META.priceChangeNotice}</p>
                  <p className="text-muted">Estado do link: {cta.reason}</p>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <div className="container-site space-y-20 py-16">
        <div className="grid gap-x-16 gap-y-12 md:grid-cols-2">
          <div>
            <h2 className="text-xl font-semibold text-ink">
              Para quem faz sentido
            </h2>
            <p className="mt-3 text-neutral-700">{offer.forWhomItMakesSense}</p>
            <h3 className="mt-5 font-semibold text-ink">Benefícios com base na ficha</h3>
            <ul className="mt-2 space-y-2">
              {offer.supportedBenefits.map((b) => (
                <li key={b} className="flex gap-2 text-sm text-neutral-700">
                  <span className="text-emerald-700" aria-hidden="true">+</span> {b}
                </li>
              ))}
            </ul>
          </div>
          <div>
            <h2 className="text-xl font-semibold text-ink">
              Compatibilidade, limitações e pontos em aberto
            </h2>
            <ul className="mt-3 space-y-2">
              {offer.limitations.map((l) => (
                <li key={l} className="flex gap-2 text-sm text-neutral-700">
                  <span className="text-amber-700" aria-hidden="true">−</span> {l}
                </li>
              ))}
            </ul>
            <h3 className="mt-5 font-semibold text-ink">O que não sabemos</h3>
            <ul className="mt-2 space-y-2">
              {offer.unknowns.map((u) => (
                <li key={u} className="flex gap-2 text-sm text-neutral-700">
                  <span className="text-neutral-400" aria-hidden="true">?</span> {u}
                </li>
              ))}
            </ul>
          </div>
        </div>

        {variants.length > 0 && (
          <section>
            <SectionHeading sub="Variações de SKU, cor, voltagem ou faixa de preço associadas ao grupo deste produto no anúncio oficial.">
              Variantes listadas no catálogo do Mercado Livre ({variants.length})
            </SectionHeading>
            <div className="variants-grid grid gap-x-10 gap-y-8 sm:grid-cols-2 lg:grid-cols-3">
              {variants.map((v) => (
                <article key={`${v.productId}-${v.sku}-${v.name}`} className="border-t border-line pt-5">
                  <div className="flex items-start justify-between gap-2">
                    <span className="font-mono text-[0.7rem] text-muted">
                      {v.productId} · SKU {v.sku}
                    </span>
                    {Number.isFinite(v.priceBrlCents) && (
                      <strong className="font-semibold text-ink tabular-nums">{formatBrl(v.priceBrlCents as number)}</strong>
                    )}
                  </div>
                  <h3 className="mt-3 font-semibold text-ink">{v.name}</h3>
                  <p className="mt-1 text-sm text-muted">
                    {v.color && (
                      <>
                        Cor: <strong>{v.color}</strong>
                      </>
                    )}
                    {v.voltage && (
                      <>
                        {v.color ? " · " : ""}Voltagem: <strong>{v.voltage}</strong>
                      </>
                    )}
                    {v.returnDays ? ` · Devolução ${v.returnFees || "grátis"} (${v.returnDays}d)` : ""}
                  </p>
                  {v.highlights && v.highlights.length > 0 && (
                    <ul className="mt-3 list-disc space-y-1 pl-5 text-sm text-neutral-700 marker:text-neutral-300">
                      {v.highlights.map((h) => (
                        <li key={h}>{h}</li>
                      ))}
                    </ul>
                  )}
                </article>
              ))}
            </div>
          </section>
        )}

        {specs.length > 0 && (
          <section>
            <SectionHeading sub="Atributos estruturados coletados da tabela oficial de especificações no momento da captura.">
              Ficha técnica extraída do anúncio ({specs.length} atributos)
            </SectionHeading>
            <div>
              <dl className="specs-grid grid gap-x-16 border-t border-line sm:grid-cols-2">
                {specs.slice(0, 14).map((sp, idx) => (
                  <div key={`${sp.name}-${idx}`} className="flex justify-between gap-6 border-b border-line py-3 text-sm">
                    <dt className="text-muted">{sp.name}</dt>
                    <dd className="text-right text-ink">{sp.value}</dd>
                  </div>
                ))}
              </dl>
              {specs.length > 14 && (
                <details className="group">
                  <summary className="cursor-pointer list-none py-4 text-sm font-medium text-ink underline decoration-neutral-300 underline-offset-4">
                    Ver todas as {specs.length} especificações ({specs.length - 14} restantes)
                    <span className="ml-1 inline-block transition-transform group-open:rotate-180" aria-hidden="true">▾</span>
                  </summary>
                  <dl className="specs-grid grid gap-x-16 border-t border-line sm:grid-cols-2">
                    {specs.slice(14).map((sp, idx) => (
                      <div key={`${sp.name}-more-${idx}`} className="flex justify-between gap-6 border-b border-line py-3 text-sm">
                        <dt className="text-muted">{sp.name}</dt>
                        <dd className="text-right text-ink">{sp.value}</dd>
                      </div>
                    ))}
                  </dl>
                </details>
              )}
            </div>
          </section>
        )}

        {offer.longDescription && (
          <section>
            <SectionHeading>Descrição completa reproduzida do anúncio</SectionHeading>
            <div className="long-desc-card">
              <div className="prose-site max-w-none">
                {offer.longDescription
                  .split(/\n{2,}/)
                  .map((b) => b.trim())
                  .filter(Boolean)
                  .map((block, idx) => (
                    <p key={idx} className="whitespace-pre-line">
                      {block}
                    </p>
                  ))}
              </div>
              {offer.shareText && (
                <div className="mt-6 border-t border-line pt-6">
                  <p className="text-sm font-medium text-ink">
                    Texto de compartilhamento oficial gerado pelo hub ({offer.shareId || ""}):
                  </p>
                  <pre
                    id={`ml-share-text-${offer.productId || offer.slug}`}
                    className="mt-3 overflow-x-auto rounded-md bg-tile p-4 font-mono text-xs leading-relaxed whitespace-pre-wrap [overflow-wrap:anywhere] text-ink"
                  >
                    {offer.shareText}
                  </pre>
                  <CopyButton className="mt-3" targetId={`ml-share-text-${offer.productId || offer.slug}`} label="Copiar mensagem oficial de divulgação" />
                </div>
              )}
              {live && (
                <div className="mt-6 border-t border-line pt-6">
                  <p className="text-sm font-medium text-ink">
                    Divulgação no Bot do Telegram ({SITE_META.telegramHandle}) e Auxiliar Humano no WhatsApp Web (sem API da Meta)
                  </p>
                  <div className="mt-4 grid gap-4 lg:grid-cols-2 [&>*]:min-w-0">
                    <div>
                      <p className="mb-2 text-xs font-semibold text-muted">Telegram Bot API (HTML · até 1.024 caracteres)</p>
                      <pre
                        id={`tg-bot-draft-${offer.slug}`}
                        className="max-h-80 overflow-auto rounded-md bg-tile p-4 font-mono text-xs leading-relaxed whitespace-pre-wrap [overflow-wrap:anywhere] text-ink"
                      >
                        {formatTelegramHtmlOffer(offer, `https://${SITE_META.domain}`)}
                      </pre>
                      <CopyButton className="mt-2" targetId={`tg-bot-draft-${offer.slug}`} label="Copiar legenda HTML do Telegram" />
                    </div>
                    <div>
                      <p className="mb-2 text-xs font-semibold text-muted">WhatsApp Web assistido (revisão humana, sem Meta API)</p>
                      <pre
                        id={`wa-web-draft-${offer.slug}`}
                        className="max-h-80 overflow-auto rounded-md bg-tile p-4 font-mono text-xs leading-relaxed whitespace-pre-wrap [overflow-wrap:anywhere] text-ink"
                      >
                        {formatWhatsappWebAssistedText(offer, `https://${SITE_META.domain}`)}
                      </pre>
                      <CopyButton className="mt-2" targetId={`wa-web-draft-${offer.slug}`} label="Copiar rascunho WhatsApp Web" />
                    </div>
                  </div>
                </div>
              )}
            </div>
          </section>
        )}

        <section>
          <SectionHeading sub={offer.sevenDayCoverageNote}>
            Histórico de observações de preço ({offer.priceObservations.length})
          </SectionHeading>
          <div className="overflow-x-auto">
            <table className="w-full min-w-[640px] text-left text-sm">
              <thead className="border-b border-line text-xs tracking-wide text-muted uppercase">
                <tr>
                  {["Data/hora (UTC)", "Variante", "Vendedor", "Base de pagamento", "Frete", "Valor"].map((h) => (
                    <th key={h} scope="col" className="px-4 py-3 font-semibold">
                      {h}
                    </th>
                  ))}
                </tr>
              </thead>
              <tbody className="divide-y divide-line">
                {offer.priceObservations.map((obs, idx) => (
                  <tr key={obs.id || idx}>
                    <td className="px-4 py-3 font-mono text-xs">{obs.observedAt}</td>
                    <td className="px-4 py-3">{obs.variantId}</td>
                    <td className="px-4 py-3">{obs.sellerName}</td>
                    <td className="px-4 py-3">{obs.paymentBasis}</td>
                    <td className="px-4 py-3 text-muted">{obs.freightNote}</td>
                    <td className="px-4 py-3 font-semibold text-ink">{formatBrl(obs.priceBrlCents)}</td>
                  </tr>
                ))}
                {offer.priceObservations.length === 0 && (
                  <tr>
                    <td colSpan={6} className="px-4 py-6 text-center text-muted">
                      Série de observações ainda em acumulação.
                    </td>
                  </tr>
                )}
              </tbody>
            </table>
          </div>
        </section>

        <section>
          <SectionHeading>Evidência e direitos de mídia</SectionHeading>
          <div className="grid gap-x-16 gap-y-10 md:grid-cols-2">
            {evidence.map((ev) => (
              <div key={ev.id} className="border-t border-line pt-5">
                <Badge>{ev.sourceType}</Badge>
                <h3 className="mt-3 font-semibold text-ink">{ev.title}</h3>
                <p className="mt-1 text-sm text-muted">
                  {ev.publisher} · Coletado em {ev.retrievedAt}
                </p>
                <ul className="mt-3 list-disc space-y-1 pl-5 text-sm text-neutral-700 marker:text-neutral-300">
                  {ev.verifiedFacts.map((f) => (
                    <li key={f}>{f}</li>
                  ))}
                </ul>
              </div>
            ))}
            <div className="border-t border-line pt-5">
              <Badge tone={rights?.rightsVerified ? "emerald" : "danger"}>
                {rights?.rightsVerified
                  ? offer.isDemoFixture
                    ? "Uso ilustrativo permitido no protótipo"
                    : "Mídia vinculada ao programa Mercado Livre Afiliados"
                  : "Direitos pendentes"}
              </Badge>
              <h3 className="mt-3 font-semibold text-ink">Registro de direitos ({rights?.id || "-"})</h3>
              <dl className="mt-2 space-y-1.5 text-sm">
                <div>
                  <dt className="inline font-semibold">Descrição: </dt>
                  <dd className="inline text-neutral-700">{rights?.assetDescription || "-"}</dd>
                </div>
                <div>
                  <dt className="inline font-semibold">Licença: </dt>
                  <dd className="inline text-neutral-700">{rights?.licenseType || "-"}</dd>
                </div>
                <div>
                  <dt className="inline font-semibold">Escopo permitido: </dt>
                  <dd className="inline text-muted">{rights?.ownerPermittedUse || "-"}</dd>
                </div>
              </dl>
            </div>
          </div>
        </section>

        {alternatives.length > 0 && (
          <section>
            <SectionHeading>Alternativas para comparar</SectionHeading>
            <div className="grid gap-x-16 md:grid-cols-2">
              {alternatives.map((alt) => (
                <MiniOfferCard key={alt.slug} offer={alt}>
                  <p className="mt-1 line-clamp-2 text-sm text-neutral-600">{alt.oneLineReason}</p>
                </MiniOfferCard>
              ))}
            </div>
          </section>
        )}

        <p className="text-center text-sm text-muted">
          <Link href="/ofertas/" className="font-semibold text-ink hover:underline">
            ← Voltar para todas as ofertas
          </Link>
        </p>
      </div>
    </>
  );
}
