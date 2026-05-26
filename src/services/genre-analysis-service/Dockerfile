# syntax=docker/dockerfile:1.7

FROM python:3.12-slim

WORKDIR /app

ENV PYTHONDONTWRITEBYTECODE=1
ENV PYTHONUNBUFFERED=1
ENV PYTHONPATH=/app

RUN apt-get update \
    && apt-get install -y --no-install-recommends bash dos2unix \
    && rm -rf /var/lib/apt/lists/*

COPY requirements.txt .

RUN --mount=type=cache,target=/root/.cache/pip \
    pip install -r requirements.txt

COPY ./proto ./proto
COPY ./src/genre_analysis_server.py .
COPY ./src/generate_proto.sh .

RUN dos2unix generate_proto.sh \
    && chmod +x generate_proto.sh \
    && PROTO_ROOT="." bash generate_proto.sh \
    && cp generated_protos/*.py .

EXPOSE 50055

CMD ["python", "genre_analysis_server.py"]