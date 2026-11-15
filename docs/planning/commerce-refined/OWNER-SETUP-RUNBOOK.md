# Zaturo Commerce — runbook do proprietário (contas e ativação)

Status: **planejamento operacional**. Este arquivo não implementa nem ativa comércio.
Ele organiza as ações que só o proprietário pode executar (contas, termos, identidade,
KYC, pagamentos e aprovação de mídia) e define o que o Codex pode fazer em cada passo.

Fontes de escopo: [plano refinado](ZATURO-COMMERCE-REFINED-PLAN.md),
[backlog](IMPLEMENTATION-BACKLOG.md) e [evidências](EVIDENCE.md).

Owner-requested next-stage planning: [Procurali business and website](PROCURALI-BUSINESS-AND-SITE.md),
[creative production instructions](PROCURALI-CREATIVE-PLAYBOOK.md) and
[Fable website/identity prompt](PROCURALI-FABLE-PROMPT.md). These specify proposed
creative/site work; they do not activate publication or replace confirmed owner decisions
in [open questions](../../OPEN_QUESTIONS.md). In particular, the current owner budget is
USD 20 per month combined across the two specified models, without a call-count limit.

## 1. Regra principal: não crie contas duplicadas

O piloto **não exige uma conta nova em cada serviço**. Na maioria dos casos é preciso
usar a conta existente e apenas se inscrever no programa de afiliados, criar um canal ou
solicitar acesso a uma API.

| Serviço | O que fazer | O que NÃO fazer |
|---|---|---|
| Telegram | Usar a conta existente; criar **um canal** e **um bot** | Criar outra conta pessoal, comprar números ou usar identidade falsa |
| Shopee | Usar a conta Shopee existente e inscrever-se no **Programa de Afiliados** | Criar uma segunda conta para burlar limites ou termos |
| Mercado Livre | Usar a conta existente, somente se for o fallback | Criar conta paralela para o mesmo negócio |
| OpenCode | Usar a assinatura Go existente | Criar segunda conta/chave para separar comércio |
| Domínio | Opcional; registrar no Registro.br com CPF/CNPJ real | Registrar domínio de terceiro ou usar identidade falsa |
| INPI | Apenas **consultar** disponibilidade agora | Depositar marca sem decisão e sem orientação |

Antes de criar qualquer coisa, o Codex deve perguntar: **“você já tem essa conta?”** Se a
resposta for sim, o passo vira login/inscrição, não cadastro novo.

## 2. Segurança e dados que nunca entram no chat

- Senhas, tokens, OTPs, códigos de verificação, CPF/CNPJ completo, dados bancários e
  comprovantes **não** vão para o chat, para o Git ou para arquivos do repositório.
- O proprietário digita dados sensíveis e resolve CAPTCHA/2FA/KYC. O Codex não faz isso.
- Tokens de bot e segredos de API ficam em arquivo privado `0600` no PC do worker.
- Use gerenciador de senhas e ative 2FA nas contas Shopee, Mercado Livre, Telegram e
  OpenCode.
- Evite VPN/proxy para falsear região; os programas de afiliados verificam identidade e
  localização.
- Termos, contratos e decisões financeiras são aceitos pelo proprietário, não pelo Codex.

## 3. Retrato do código antes de começar

- F1–F7 estão implementados **somente** no branch isolado `codex/autonomous-commerce`
  (worktree `~/.codex/worktrees/autonomous-commerce/ZaturoBot`), ainda não integrado à
  `main`.
- F8 (painel Linux/Android) **não existe**. O app 0.8 não tem tela de comércio.
- F4 (Telegram) e F7 (API de afiliados) têm apenas código/limite offline; **nenhum smoke
  real e nenhum acesso oficial** foram feitos.
- Sem merge, deploy do Hub schema 10 e configuração do worker, as contas criadas não
  publicam nada.

## 4. Fase 0 — ficha do proprietário (não secreta)

Preencha esta ficha em um arquivo local privado; não é necessário colar no chat.

| Campo | Valor |
|---|---|
| Marca escolhida | |
| Handle do Telegram | `@` |
| Domínio (opcional) | |
| Programa escolhido | Shopee / Mercado Livre |
| Conta Shopee já existe? | sim/não |
| Conta Mercado Livre já existe? | sim/não (fallback) |
| Conta Telegram já existe? | sim/não |
| Assinatura OpenCode Go ativa? | sim/não |
| Telegram aceito como mídia pela Shopee? | sim/não/em análise |
| `sub_ID` disponível no gerador de links? | sim/não |
| Acesso à API Shopee | não solicitado / em análise / aprovado / recusado |
| Faixa de preço | R$ 30–300 (default) |
| Posts por dia/janelas | 2/dia, 11:30–12:30 e 19:00–21:00 |
| Limite de IA | ≤10% da franquia Go, ≤20 chamadas/dia |
| Aceita os defaults O-01…O-09? | sim/não |

## 5. Fase 1 — marca e disponibilidade (O-01)

**Objetivo:** confirmar nome, handle e domínio antes de criar canal ou pedir API.

1. Teste o handle do Telegram em `https://t.me/<handle>`. Se abrir um perfil/canal
   existente, escolha outro.
2. Consulte o domínio em https://registro.br/. Registro é opcional para o piloto.
3. Consulte a marca em https://servicos.busca.inpi.gov.br/. Isto é consulta, não depósito.
4. Registre a decisão na ficha e em `docs/OPEN_QUESTIONS.md` (seção de comércio).

**O que o Codex pode fazer:** abrir as páginas, verificar se o handle/domínio está livre,
registrar o resultado não secreto e sugerir alternativas.
**O que o proprietário faz:** decide o nome e, se quiser, paga o registro do domínio.

**Critério de aceite:** marca e handle escolhidos; handle não pertence a terceiro; decisão
registrada.

## 6. Fase 2 — Telegram: canal + bot

**Objetivo:** ter um canal público de publicação e um canal privado de teste, com um bot
administrador que só pode postar.

1. Na conta Telegram existente, crie um **canal público** com o handle decidido.
2. Crie um **canal privado** para o primeiro smoke (não use o canal público para teste).
3. Abra https://t.me/BotFather, execute `/newbot`, escolha nome e username do bot.
4. Guarde o token **somente** no gerenciador de senhas; não cole no chat.
5. Adicione o bot como administrador do canal público e do canal de teste, com a permissão
   mínima de **postar mensagens**.
6. Anote o `channel_id` público (`@handle`) e o identificador do canal privado.
7. Teste manualmente: envie uma mensagem no canal de teste e confirme que o bot aparece
   como administrador.

**O que o Codex pode fazer:** abrir o BotFather, ler as instruções, verificar os campos,
conferir o handle do canal e registrar o status. Pode orientar a adicionar o bot.
**O que o proprietário faz:** login, 2FA, criação do canal, token, adicionar administrador.
**Nunca:** colar o token no chat, no Git ou no histórico do shell.

**Critério de aceite:** canal público e canal de teste existem; bot é administrador com
direito de postar; token guardado em local privado; nenhum token no repositório.

Links: [BotFather](https://t.me/BotFather),
[criar canal](https://telegram.org/faq#q-how-do-i-create-a-channel),
[Bot API](https://core.telegram.org/bots/api),
[termos de bots](https://telegram.org/tos/bot-developers).

## 7. Fase 3 — Shopee Afiliados (programa principal)

**Objetivo:** inscrever a conta existente no programa, cadastrar o Telegram como mídia,
verificar `sub_ID` e solicitar acesso à API.

1. Entre em https://affiliate.shopee.com.br/ com a conta Shopee existente.
2. Conclua a inscrição de afiliado com os dados reais solicitados (KYC/CPF/CNPJ e dados de
   recebimento são digitados pelo proprietário).
3. Leia os [termos oficiais](https://help.shopee.com.br/portal/article/76443-Programa-de-Afiliados-da-Shopee---Termos-e-Condi%c3%a7%c3%b5es).
4. No painel, procure **Canais / Mídias / Meus canais** (o nome pode variar) e cadastre o
   canal público do Telegram.
5. Se Telegram não estiver disponível, abra atendimento e pergunte por escrito se o canal
   pode ser aprovado como mídia. Guarde a resposta.
6. No gerador de links, verifique se existe o campo **`sub_ID`**. Registre sim/não.
7. Gere um link de teste e confirme que ele abre o produto oficial sem redirecionador
   externo.
8. Baixe um export de ofertas e um de comissões/pedidos. Mantenha os arquivos privados.
9. Solicite acesso à API em https://affiliate.shopee.com.br/open_api. O endpoint BR é
   `https://open-api.affiliate.shopee.com.br/graphql`; não presuma aprovação.
10. Registre na ficha: mídia aprovada, `sub_ID`, status da API.

**O que o Codex pode fazer:** abrir o portal, localizar as seções, listar os campos e
requisitos, conferir se o link é oficial, registrar os fatos não secretos e preparar o CSV
no formato do importador.
**O que o proprietário faz:** login, KYC, termos, dados bancários, aprovação de mídia,
CAPTCHA, envio final e download dos relatórios.

**Regras:** um programa por canal; não misture Shopee e Mercado Livre no mesmo canal; não
edite links oficiais; não use encurtadores.

**Critério de aceite:** conta de afiliado ativa; canal aprovado (ou resposta oficial);
`sub_ID` verificado; API solicitada ou aprovada; um link oficial de teste válido.

## 8. Fase 4 — Mercado Livre (somente fallback)

Só execute se a Shopee recusar o Telegram como mídia.

1. https://www.mercadolivre.com.br/l/afiliados-home
2. https://www.mercadolivre.com.br/afiliados/registration
3. Leia https://www.mercadolivre.com.br/ajuda/30228.
4. Declare o canal do Telegram como mídia no cadastro/portal.
5. Gere links manualmente no painel; não há API pública de afiliados.
6. Exporte comissões/pedidos quando disponível.

**Critério de aceite:** conta existente inscrita; canal declarado; link manual oficial;
status registrado.

## 9. Fase 5 — OpenCode Go

**Objetivo:** confirmar a franquia e separar o limite do comércio sem criar conta nova.

1. Entre em https://opencode.ai/go com a conta existente.
2. Confirme o plano Go, a franquia e que **Extra Usage está desligado**.
3. Leia https://opencode.ai/docs/go/ e os [termos](https://opencode.ai/legal/terms-of-service).
4. Decida o limite do comércio (default: ≤10% da franquia, ≤20 chamadas/dia).
5. Não crie chave nova; a chave existente continua local no worker.
6. O perfil `config/commerce-models.example.toml` permanece **desligado** até o merge.

**Critério de aceite:** assinatura ativa; Extra Usage off; limite decidido; nenhuma chave
copiada para o repositório.

## 10. Fase 6 — handoff para o Codex (sem segredos)

Entregue ao Codex apenas fatos não secretos:

- marca, handle, canal público, canal de teste (nomes públicos);
- programa escolhido e status da mídia;
- `sub_ID`: sim/não;
- API: não solicitada/em análise/aprovada/recusada;
- limites e decisões O-01…O-09;
- caminho local privado dos CSVs (o Codex lê o formato, não publica conteúdo sensível).

Peça ao Codex para atualizar `docs/OPEN_QUESTIONS.md`, a ficha e a configuração de exemplo
**sem** inserir tokens. Depois, o Codex pode executar a parte de código.

## 11. Fase 7 — trabalho de código no Codex

1. Integrar `codex/autonomous-commerce` com a `main` `45dc9ac`.
2. Resolver os 20 conflitos; renumerar ADRs 0022–0025 para 0024–0027.
3. Rodar `cargo xtask check --locked` e `cargo xtask secrets --locked`.
4. Implementar F8: painel Linux/Android de comércio.
5. Preparar deploy do Hub schema 10 com backup consistente.
6. Configurar o worker com `[telegram]` e marca, sem habilitar `allow_execute` antes do
   smoke.
7. Só então rodar o smoke no canal privado e importar a primeira oferta real.

**O que o Codex pode fazer:** editar código, resolver conflitos, rodar testes, preparar
configs e documentação.
**O que o proprietário autoriza:** push/PR, deploy, restart de serviço, publicação real.

## 12. Fase 8 — primeiro dado real e piloto

1. Importar 1–5 ofertas oficiais no formato do importador.
2. Criar política/campanha em `draft`; aprovar a política explicitamente.
3. Gerar um lote de rascunhos e revisar texto, preço, link e `#publi`.
4. Aprovar o lote exato por hash.
5. Publicar **um** post no canal de teste; conferir recibo e mensagem.
6. Stage A: 3 dias só rascunhos, revisão manual.
7. Stage B: 14 dias com aprovação em lote; só depois considerar `auto`.
8. Importar o relatório de comissões; receita não aprovada continua “pendente”, não zero.

## 13. O que o Codex pode e não pode fazer

| Pode | Não pode |
|---|---|
| Abrir páginas oficiais e ler requisitos | Criar conta, aceitar termos ou clicar no envio final |
| Listar campos e conferir dados não secretos | Digitar senha, OTP, CPF/CNPJ, dados bancários ou cartão |
| Ajudar a preencher campos não sensíveis, com o dono presente | Resolver CAPTCHA/2FA/KYC no lugar do dono |
| Verificar se um link é oficial e preservar os bytes | Editar links, usar encurtador ou afiliado de terceiro |
| Registrar evidências não secretas | Guardar tokens, segredos ou dados pessoais no repositório |
| Preparar CSVs, configs e testes | Publicar, pagar, assinar ou contratar em nome do dono |
| Sugerir alternativas de marca/handle | Registrar marca/domínio sem autorização explícita |

## 14. Prompts prontos para colar no Codex

**Prompt mestre:**

```text
Leia docs/planning/commerce-refined/OWNER-SETUP-RUNBOOK.md e conduza o proprietário
passo a passo. Trabalhe em uma etapa por vez. Antes de abrir qualquer cadastro, pergunte
se a conta já existe e não crie duplicata. Nunca peça senha, OTP, CPF/CNPJ, dados
bancários ou token no chat. Pare antes de qualquer botão de criar conta, aceitar termos,
pagar ou enviar formulário; o proprietário faz essa ação. Registre apenas fatos não
secretos e não altere código de produção sem autorização.
```

**Fase 1:**

```text
Vamos decidir a marca. Verifique a disponibilidade de https://t.me/<handle>,
https://registro.br/ e https://servicos.busca.inpi.gov.br/. Não registre nada.
Liste os resultados e sugira 3 handles alternativos se estiver ocupado.
```

**Fase 2:**

```text
Vamos criar o canal e o bot do Telegram. Abra o BotFather, leia as instruções e me
guie. Não digite meu token, não clique em criar conta e não aceite termos por mim.
No fim, confirme apenas: canal público, canal de teste, bot admin com permissão de postar.
```

**Fase 3:**

```text
Abra https://affiliate.shopee.com.br/ e verifique se minha sessão existente está ativa.
Se não estiver, pare e peça para eu entrar. Depois localize a inscrição de afiliado,
o cadastro de mídia/canal e o gerador de links. Liste os campos e requisitos, não envie
nada. Verifique especificamente se Telegram é aceito como mídia e se existe campo sub_ID.
```

**Fase 6:**

```text
Estes são os fatos não secretos do setup: [cole a ficha sem segredos].
Atualize docs/OPEN_QUESTIONS.md e prepare a configuração de exemplo do comércio.
Não insira tokens, não habilite allow_execute e não faça push/deploy.
```

## 15. Critérios de aceite do setup

- [ ] Marca/handle decididos e disponíveis; decisão registrada.
- [ ] Canal público e canal privado de teste existem.
- [ ] Bot é administrador com permissão apenas de postar; token fora do repositório.
- [ ] Conta Shopee existente inscrita no programa de afiliados.
- [ ] Canal Telegram cadastrado como mídia ou resposta oficial registrada.
- [ ] `sub_ID` verificado (sim/não) e link oficial de teste válido.
- [ ] API Shopee solicitada ou status registrado.
- [ ] OpenCode Go confirmado; Extra Usage desligado; limite decidido.
- [ ] Nenhum segredo em chat, Git, CSV ou configuração de exemplo.
- [ ] Código integrado (merge/F8), gates passando, deploy autorizado.
- [ ] Smoke real no canal privado concluído antes do canal público.

## 16. Condições de parada

Pare e peça decisão do proprietário se:

- o site pedir CPF/CNPJ, documento, dados bancários, cartão ou pagamento;
- aparecer CAPTCHA, 2FA, KYC ou verificação por telefone;
- a conta existente estiver bloqueada ou o programa exigir conta nova;
- os termos entrarem em conflito com o uso de Telegram/automação;
- a Shopee negar a mídia ou a API;
- houver dúvida sobre identidade, marca ou titularidade;
- qualquer passo exigir publicar, enviar mensagem ou aceitar contrato em nome do dono.
