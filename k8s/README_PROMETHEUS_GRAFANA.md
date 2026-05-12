# Grafana + Prometheus no BookCloud

Este documento resume como abrir o Grafana, consultar métricas no Prometheus e testar os serviços monitorizados do BookCloud.

## Serviços monitorizados

Atualmente, o Prometheus recolhe métricas de:

```txt
author-catalog
book-catalog
rating-catalog
```

Cada serviço expõe métricas em:

```
/metrics
```

Na porta:

```
9100
```

1. Verificar se o monitoring está ativo

```
kubectl -n monitoring get pods
kubectl -n monitoring get svc
```

Verificar os `ServiceMonitor`:

```
kubectl -n bookcloud get servicemonitor
```

Esperado:

```
author-catalog
book-catalog
rating-catalog
```

2. Abrir Grafana

```
kubectl -n monitoring port-forward service/monitoring-grafana 3000:80
```

Abrir no browser:

```
http://localhost:3000
```

Credenciais padrão:

```
user: admin
password: admin
```

3. Abrir Prometheus

```
kubectl -n monitoring port-forward service/monitoring-kube-prometheus-prometheus 9090:9090
```

Abrir no browser:

```
http://localhost:9090
```

Página de targets:

```
http://localhost:9090/targets
```

Nessa página devem aparecer targets do namespace:

```
bookcloud
```

4. Testar métricas diretamente
Book Catalog

```
kubectl -n bookcloud port-forward service/book-catalog 9101:9100
curl http://localhost:9101/metrics
```

Author Catalog

```
kubectl -n bookcloud port-forward service/author-catalog 9102:9100
curl http://localhost:9102/metrics
```

Rating Catalog

```
kubectl -n bookcloud port-forward service/rating-catalog 9103:9100
curl http://localhost:9103/metrics
```

5. Queries principais no Grafana
No Grafana, abrir:

```
Explore
```

Selecionar a datasource:

```
Prometheus
```

Ver serviços ativos

```
up{namespace="bookcloud"}
```

Ver apenas os catálogos monitorizados

```
up{namespace="bookcloud", service=~"author-catalog|book-catalog|rating-catalog"}
```

Total de requests por serviço, operação e status

```
sum by (service, operation, status) (bookcloud_requests_total)
```

Total de requests por serviço

```
sum by (service) (bookcloud_requests_total)
```

Requests por segundo nos últimos 5 minutos

```
sum by (service) (rate(bookcloud_requests_total[5m]))
```

Requests por operação

```
sum by (service, operation) (rate(bookcloud_requests_total[5m]))
```

Latência média

```
sum by (service, operation) (
  rate(bookcloud_request_duration_seconds_sum[5m])
)
/
sum by (service, operation) (
  rate(bookcloud_request_duration_seconds_count[5m])
)
```

6. Gerar métricas para teste
Abrir o Kong localmente:

```
kubectl -n bookcloud port-forward service/kong 9000:80
```

Fazer pedidos à API:

```
curl http://localhost:9000/api/books
curl http://localhost:9000/api/authors
curl http://localhost:9000/api/ratings
```

Depois voltar ao Grafana e testar:

```
bookcloud_requests_total
```

7. Verificar labels dos Services
Os Services precisam ter labels para o Prometheus os descobrir:

```
kubectl -n bookcloud get svc author-catalog book-catalog rating-catalog --show-labels
```

Esperado:

```
author-catalog   app=author-catalog
book-catalog     app=book-catalog
rating-catalog   app=rating-catalog
```

8. Verificar portas dos Services

```
kubectl -n bookcloud get svc author-catalog book-catalog rating-catalog -o yaml | grep -A20 "ports:"
```

Cada serviço deve ter:

```
- name: metrics
  port: 9100
  targetPort: 9100
```

9. Problemas comuns
`up{namespace="bookcloud"}` dá `No data`
Verificar:

```
kubectl -n bookcloud get servicemonitor
kubectl -n bookcloud get svc author-catalog book-catalog rating-catalog --show-labels
kubectl -n monitoring port-forward service/monitoring-kube-prometheus-prometheus 9090:9090
```

Depois abrir:

```
http://localhost:9090/targets
```

`bookcloud_requests_total` dá `No data`
Fazer pedidos primeiro:

```
curl http://localhost:9000/api/books
curl http://localhost:9000/api/authors
curl http://localhost:9000/api/ratings
```

Depois testar novamente no Grafana:

```
bookcloud_requests_total
```

10. Comandos principais

```
kubectl -n monitoring get pods
kubectl -n monitoring get svc
kubectl -n bookcloud get servicemonitor
kubectl -n monitoring port-forward service/monitoring-grafana 3000:80
kubectl -n monitoring port-forward service/monitoring-kube-prometheus-prometheus 9090:9090
kubectl -n bookcloud port-forward service/kong 9000:80
```
