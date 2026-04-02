FROM python:3.12-slim

WORKDIR /app

COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

COPY ./proto ./proto
COPY ./src/genre_analysis_server.py .
COPY ./src/generate_proto.sh .


RUN apt-get update \
 && apt-get install -y --no-install-recommends bash dos2unix \
 && rm -rf /var/lib/apt/lists/* \
 && dos2unix generate_proto.sh \
 && chmod +x generate_proto.sh \
 && PROTO_ROOT="/app" bash generate_proto.sh \
 && cp generated_protos/*.py .

ENV PYTHONPATH=/app

EXPOSE 50055

CMD ["python", "genre_analysis_server.py"]
>>>>>>> Stashed changes
