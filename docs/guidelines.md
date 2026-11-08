# Regras de trabalho

Como se desenvolve e integra código neste projeto.

## Branches

- O `main` é o único branch permanente. Todo o trabalho chega ao `main` através de um Pull Request.
- Cada assunto tem o seu branch temporário, criado a partir do `main` atualizado. Esse branch é apagado depois do merge.
- Uma experiência também vai para um branch temporário. Se der resultado, segue para PR; se não interessar, apaga-se o branch.
- Os branches devem durar pouco. Se um branch se arrastar, traz-se o `main` para ele de vez em quando com `git merge main`.
- O nome do branch começa por um prefixo que indica o tipo de trabalho:

  | Prefixo | Uso |
  |---|---|
  | `feature/` | funcionalidade nova |
  | `fix/` | correção de erro |
  | `maintenance/` | dependências, refatorização, limpeza |
  | `packaging/` | compilação e distribuição |
  | `docs/` | documentação |

### Porque não há um branch `devel`

Decidido a 8 de novembro de 2026, quando o branch `devel` foi abandonado.

- O projeto tem uma só pessoa a desenvolver, por isso um branch de integração só acrescenta passos.
- Dois branches permanentes têm de ser sincronizados nos dois sentidos, e é fácil esquecer. Uma correção do tema da tabela de dados ficou sete meses no `devel` sem chegar ao `main`.
- O CI só verifica o `main` e os PRs para o `main`.

Esta decisão deve ser revista se o projeto passar a ter várias pessoas a contribuir, ou se for preciso juntar várias alterações para as testar em conjunto antes de uma versão.

## Antes de cada commit

```powershell
cargo fmt --all
cargo test
```

O CI rejeita código que não esteja formatado com `cargo fmt`.

## Mensagens de commit

- Escritas em português.
- O título usa o infinitivo, sem ponto final, com 72 caracteres no máximo. Por exemplo: `Corrigir o tema da tabela de dados`.
- O corpo, separado por uma linha em branco, explica o que muda e porquê.
- Cada commit trata de um só assunto. Por exemplo, a formatação automática fica num commit separado das alterações de código.

## Pull Requests

1. Abrir o PR para o `main`.
2. Esperar que o CI fique verde: formatação, compilação e testes.
3. Fazer o merge com **Create a merge commit**, para manter os commits do branch tal como estão.
4. Apagar o branch, no GitHub e localmente:

   ```powershell
   git switch main
   git pull
   git branch -d <branch>
   git fetch --prune
   ```
