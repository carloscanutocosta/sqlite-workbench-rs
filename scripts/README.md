# Scripts de Utilidade — SQLite Workbench RS

Este diretório contém scripts PowerShell para automatizar tarefas de compilação, backup e restauro do ambiente de trabalho.

## Compilação (Build)

### `build_release.ps1`
Gera o executável otimizado para produção.
- Aplica LTO (Link Time Optimization) e *strip* para reduzir significativamente o tamanho do binário.
- Copia o executável resultante para a pasta `dist/` com um carimbo de data/hora (*timestamp*).
- Efetua a limpeza automática da pasta `dist/`, mantendo apenas os últimos 5 binários gerados.

### `build_debug.ps1`
Gera o executável de desenvolvimento.
- Compilação rápida sem otimizações de performance.
- Ideal para depuração e ciclos rápidos de desenvolvimento.

---

## Gestão de Dados (Backup & Restore)

Estes scripts gerem a salvaguarda dos teus dados de utilizador (`settings.json`, `favorites.json`, `history.json`) e do binário de release.

### `backup.ps1`
Cria uma cópia de segurança completa do estado atual.
- **Caminho por omissão:** `D:\Backup\sqlite_workbench`
- **Execução básica:** `.\scripts\backup.ps1`
- **Parâmetros:** Podes especificar um destino customizado usando `.\scripts\backup.ps1 -Path "C:\MeusBackups"`.
- **Retenção:** Remove automaticamente backups com mais de 30 dias.
- **Validação:** Verifica se o disco de destino está montado antes de iniciar.

### `restore.ps1`
Permite selecionar e repor um backup guardado anteriormente de forma interativa.
- **Segurança:** Antes de aplicar qualquer restauro, o script realiza um backup automático do estado atual ("Safety Backup").
- **Uso interativo:** Lista os backups disponíveis com informação de data e número de ficheiros.
- **Parâmetros:** Aceita o parâmetro `-Path` para localizar backups em pastas personalizadas.

---

## Logs e Manutenção

- **operations.log:** Regista o sucesso de cada operação, o utilizador que a realizou e o timestamp.
- **Rotação de Logs:** Se o ficheiro de log exceder 1MB, é movido para `operations.old.log` para evitar consumo excessivo de espaço.
- **Metadados:** Cada pasta de backup contém um ficheiro `backup-meta.json` com informações técnicas sobre a origem da salvaguarda.

## Notas de Execução

- Certifica-te que a política de execução do PowerShell permite scripts (`Set-ExecutionPolicy RemoteSigned -Scope CurrentUser`).
- Os scripts de build requerem que o `cargo` (Rust) esteja instalado e acessível no PATH.

---
*Último build realizado em:* ---

---
*© 2025 NORMAXIS — SQLite Workbench RS*