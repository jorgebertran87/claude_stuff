FROM node:20-alpine

WORKDIR /app

COPY package*.json ./
RUN npm install

COPY tsconfig.json jest.config.js ./
COPY src/ ./src/
COPY tests/ ./tests/

ENTRYPOINT ["npm", "test"]
