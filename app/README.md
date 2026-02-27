## Now BTC is availabe (Global API)

### How to run docker to store BTC data on Clickhouse DB

1. First start You docker on Windows
2. CD to project directory ( cd ./Dockerized_Services )

3. Run codes

```bash
docker compose build --no-cache
```

```bash
APP_MODE=btc docker compose up -d
```

4. See Log is Running Now ...

```bash
docker compose logs -f
```

### Intract with Clickhouse Database

```bash
docker exec -it clickhouse clickhouse-client
```

And then intract with database:
```sql
show databases;
use btc_db;
show tables;
select * from wallet_info;
```