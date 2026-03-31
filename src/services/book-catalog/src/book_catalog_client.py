import grpc
import book_catalog_pb2
import book_catalog_pb2_grpc
import common_pb2

def run():
    with grpc.insecure_channel('localhost:50051') as channel:
        stub = book_catalog_pb2_grpc.BookCatalogGrpcStub(channel)

        #health = stub.HealthCheck(common_pb2.HealthCheckRequest())
        #print('HealthCheck:', health)

        response = stub.GetBooks(book_catalog_pb2.GetBooksRequest())
        print('GetBooks:', response)

if __name__ == '__main__':
    run()