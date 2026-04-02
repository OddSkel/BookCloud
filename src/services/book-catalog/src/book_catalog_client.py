import os
import sys

ROOT_DIR = os.path.dirname(__file__)
sys.path.insert(0, ROOT_DIR)
sys.path.insert(0, os.path.join(ROOT_DIR, "generated_protos"))

import grpc
import generated_protos.book_catalog_pb2
import generated_protos.book_catalog_pb2_grpc
import generated_protos.common_pb2

def run():
    with grpc.insecure_channel('localhost:50051') as channel:
        stub = generated_protos.book_catalog_pb2_grpc.BookCatalogGrpcStub(channel)

        response = stub.GetBooks(generated_protos.book_catalog_pb2.GetBooksRequest())
        print('GetBooks:', response)

if __name__ == '__main__':
    run()