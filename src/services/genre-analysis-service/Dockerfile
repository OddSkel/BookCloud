FROM python:3.12-slim

WORKDIR /app

COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

COPY ./proto ./proto
COPY ./src/genre_analysis_server.py .
COPY ./src/generate_proto.sh .

RUN apt-get update \
 && apt-get install -y --no-install-recommends bash \
 && rm -rf /var/lib/apt/lists/* \
 && chmod +x generate_proto.sh \
 && PROTO_ROOT="." bash generate_proto.sh \
 && cp generated_protos/*.py .

ENV PYTHONPATH="/app/generated_protos:${PYTHONPATH:-}"

EXPOSE 50055

CMD ["python", "genre_analysis_server.py"]
