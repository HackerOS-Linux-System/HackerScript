CC = gcc
CFLAGS = -Wall -O2 -fPIC

all: libhsgc.a hsgc.o

libhsgc.a: hsgc.o
	ar rcs libhsgc.a hsgc.o

hsgc.o: hsgc.c hsgc.h
	$(CC) $(CFLAGS) -c hsgc.c -o hsgc.o

clean:
	rm -f *.o *.a
