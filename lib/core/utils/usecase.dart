abstract class Usecase<type, Params> {
  Future<type> call({Params params});
}
