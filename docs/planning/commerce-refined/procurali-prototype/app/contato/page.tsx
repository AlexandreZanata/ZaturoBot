import { SITE_META } from "@/data/demo-catalog.mjs";
import { pageMetadata } from "@/lib/seo";
import { Notice, buttonClass } from "@/components/site/primitives";
import { TrustPage } from "@/components/site/trust-page";

export const metadata = pageMetadata({
  route: "/contato/",
  title: "Contato",
  description: "Rotas de contato e formulário demonstrativo não conectado do Procurali.",
});

const field =
  "w-full rounded-md border border-line bg-white px-3 py-2.5 text-sm text-ink outline-none transition focus:border-ink";

export default function ContactPage() {
  return (
    <TrustPage
      route="/contato/"
      crumb="Contato"
      ldName="Contato"
      kicker="Canal de contato · Protótipo"
      title="Contato e correções editoriais"
      lead={
        <>
          Encontrou um preço desatualizado, erro de compatibilidade em um guia ou dúvida sobre transparência? Enquanto o domínio e o
          e-mail de suporte não são ativados pelo proprietário, acompanhe os avisos públicos pelo canal{" "}
          <a href={SITE_META.telegramUrl} target="_blank" rel="noopener" className="text-ink underline decoration-neutral-300 underline-offset-4">
            {SITE_META.telegramHandle}
          </a>
          .
        </>
      }
    >
      <Notice variant="warning">
        <p>
          <strong>Formulário demonstrativo:</strong> nenhum backend de envio está conectado neste protótipo estático. O botão de envio
          permanece desabilitado para evitar falsa confirmação de recebimento.
        </p>
      </Notice>
      <form id="contact-demo-form" aria-label="Formulário de contato em demonstração" className="mt-6 space-y-4 rounded-lg border border-line bg-white p-6">
        <div>
          <label htmlFor="c-name" className="mb-1.5 block text-sm font-semibold text-ink">
            Seu nome ou apelido
          </label>
          <input id="c-name" name="name" type="text" placeholder="Opcional" className={field} />
        </div>
        <div>
          <label htmlFor="c-topic" className="mb-1.5 block text-sm font-semibold text-ink">
            Assunto
          </label>
          <select id="c-topic" name="topic" className={field}>
            <option>Correção de ficha técnica ou preço</option>
            <option>Dúvida sobre transparência / afiliados</option>
            <option>Reportar link de grupo expirado</option>
          </select>
        </div>
        <div>
          <label htmlFor="c-msg" className="mb-1.5 block text-sm font-semibold text-ink">
            Mensagem
          </label>
          <textarea id="c-msg" name="message" rows={5} placeholder="Não informe CPF, senhas ou dados bancários." className={field} />
        </div>
        <button type="button" disabled aria-disabled="true" className={buttonClass("disabled", "w-full")}>
          Enviar mensagem (desabilitado — sem backend configurado)
        </button>
      </form>
    </TrustPage>
  );
}
