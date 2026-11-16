# Guia de Identidade Visual e Arquitetura de Vitrine — Procurali

- **Marca:** Procura Ali · **Wordmark:** `procurali`
- **Proposta de tagline:** “Achados que fazem sentido.” *(sujeita à aprovação final do proprietário)*
- **Versão do sistema:** `0.3.0-catalog-clarity`

## Símbolo e Favicon Oficiais

- **Favicon oficial do navegador:** [`brand/procurali_icone_branco_transparente.svg`](brand/procurali_icone_branco_transparente.svg) (`viewBox="138 178 1000 1000"`, traço branco com recorte interno de etiqueta/P em fundo transparente).
- **Ícone oficial no site:** mesmo símbolo geométrico aplicado como `brand/symbol-mark.svg` em verde escuro `#143D2B` sobre o cabeçalho claro e `#FFFFFF` sobre superfícies escuras (`#143D2B`).
- **Regra de uso:** não adicionar mascotes, carrinhos genéricos, selos falsos ou elementos visuais de Shopee/AliExpress/Mercado Livre ao símbolo.

## Paleta Funcional e Contraste

Cada cor desempenha uma função consistente na leitura do catálogo (validado para contraste WCAG AA >= 4,5:1 em textos):

| Papel funcional | Cor hex | Combinação e regra de uso |
| --- | --- | --- |
| Identidade da marca e navegação | `#143D2B` | Logotipo, filtros ativos, links de seção, rodapé escuro (`#FFFFFF` sobre `#143D2B`). |
| Ação principal de oferta | `#C2410C` / `#9A3412` hover | Botões que levam ao produto (`Ver oferta na Shopee`, `Ver oferta no Mercado Livre`) com texto `#FFFFFF`. |
| Superfície e fundo do catálogo | `#F7F8FA` e `#FFFFFF` | Canvas neutro claro `#F7F8FA`; cards `#FFFFFF` com borda discreta `#E5E7EB`. |
| Preços e títulos principais | `#171717` e `#5F6368` | Preço em destaque grafite (`28px` `800`); título (`16px` `600`); metadados em `#5F6368`. |
| Economia confirmada | `#15803D` sobre `#F0FDF4` | Selo de cupom ativo ou economia documentada em amostra de 7 dias. |
| Prazo real ou limitação | `#B91C1C` sobre `#FEF2F2` | Vencimento confirmado de cupom ou restrição verificada; nunca usar para falsas contagens regressivas. |
| Registro vencido (>30 min) | `#92400E` sobre `#FEF3C7` | Estado `Preço em atualização`, pausando botão comercial laranja até nova conferência. |
| Destaque pontual da marca | `#A3E635` | Apenas sobre `#143D2B` com texto escuro `#143D2B` (nunca em fundos claros nem para corpo de texto). |

## Tipografia, Espaçamento e Hierarquia de Vitrine

- **Fonte:** `'Inter', 'Noto Sans', system-ui, sans-serif` (leitura limpa e numerais tabulares no preço).
- **Escala tipográfica:** título do produto `16px` (`600`), preço principal `28px` (`800`), condição de pagamento `14px` logo abaixo do preço, uso e metadados `12–14px`.
- **Geometria:** escala de espaçamento `4 / 8 / 12 / 16 / 24 / 32 px`; cards com raio `12px`, botões principais com raio `8px` e altura mínima `48px`, botão de favoritar com alvo mínimo `44×44 px`.
- **Primeira dobra:** abertura compacta em fundo branco + filtros essenciais + vitrine imediata de 4 ofertas no desktop; faixa institucional movida para baixo dos primeiros produtos e grupos Telegram/WhatsApp posicionados como ação secundária.
