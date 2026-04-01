<<<<<<< Updated upstream
=======
FROM python:3.12-slim

WORKDIR /app

COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

COPY ./proto ./proto
COPY ./src/genre_analysis_server.py .
COPY ./src/generate_proto.sh .

RUN sed -i 's/\r$//' generate_proto.sh \
 && chmod +x generate_proto.sh \
 && PROTO_ROOT=/app /bin/sh generate_proto.sh \
 && cp generated_protos/*.py .

ENV PYTHONPATH=/app

EXPOSE 50055

CMD ["python", "genre_analysis_server.py"]
>>>>>>> Stashed changes
