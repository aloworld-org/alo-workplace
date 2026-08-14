FROM python:3.13-slim
ENV PYTHONDONTWRITEBYTECODE=1 PYTHONUNBUFFERED=1
WORKDIR /app
COPY products/meet/alo-transcriber/requirements.txt ./requirements.txt
RUN pip install --no-cache-dir -r requirements.txt
COPY products/meet/alo-transcriber/transcriber.py ./transcriber.py
CMD ["python", "transcriber.py", "start"]
