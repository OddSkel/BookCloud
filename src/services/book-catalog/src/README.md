# Book Catalog (Python + gRPC)

Reimplementação do serviço `book-catalog` em Python com gRPC, paridade de endpoints com a versão Rust:
- HealthCheck
- GetBooks
- GetBook
- AddBook
- UpdateBook
- DeleteBook

## Dependências

```bash
python -m pip install -r requirements.txt
```

## Geração de stubs gRPC

```bash
cd src/services/book-catalog/python
bash generate_proto.sh
```

## Execução

Configurar `.env` ao lado de `src/services/book-catalog/.env` (ou copiá-lo):

```env
POSTGRES_USER=<user>
POSTGRES_PASSWORD=<pass>
POSTGRES_DB=<db>
POSTGRES_HOST=<host>
POSTGRES_PORT=5432
GRPC_PORT=50051
SERVICE_NAME=book-catalog-python
```

```bash
cd src/services/book-catalog/python
python book_catalog_server.py
```

## Observações

- Banco usado: PostgreSQL
- A tabela `books` é criada automaticamente se não existir
- A API gRPC usa o mesmo contrato do proto `book_catalog.proto` e `common.proto`
